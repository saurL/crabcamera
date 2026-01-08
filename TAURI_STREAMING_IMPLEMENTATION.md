# Guide d'Implémentation du Streaming Caméra dans Tauri

Ce guide explique comment implémenter le streaming vidéo dans votre application Tauri avec CrabCamera, avec ou sans WebRTC, en maximisant la logique côté Rust.

## Table des Matières

1. [Architecture](#architecture)
2. [Implémentation Rust](#implémentation-rust)
3. [Commandes Tauri](#commandes-tauri)
4. [Intégration Frontend](#intégration-frontend)
5. [Exemples Complets](#exemples-complets)

---

## Architecture

### Deux Modes de Streaming

1. **Mode WebRTC** : Streaming H.264 encodé via RTP pour le web
2. **Mode Direct** : Streaming de frames RGB brutes via callback

### Flux de Données

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Tauri                         │
├─────────────────────────────────────────────────────────────┤
│  Frontend (JS)                                               │
│  ┌──────────────┐         ┌──────────────┐                  │
│  │ Start Stream │────────▶│ Stop Stream  │                  │
│  └──────────────┘         └──────────────┘                  │
│         │                         │                          │
│         ▼                         ▼                          │
├─────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                              │
│  ┌────────────────────────────────────────────────────┐     │
│  │         Unified Streaming Manager                   │     │
│  │  ┌──────────────┐      ┌──────────────────────┐   │     │
│  │  │ Camera Init  │──────▶│  Frame Callback      │   │     │
│  │  │ (optimized)  │      │  (hook hook)         │   │     │
│  │  └──────────────┘      └──────────────────────┘   │     │
│  │         │                        │                 │     │
│  │         ├────────────┬───────────┤                 │     │
│  │         ▼            ▼           ▼                 │     │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────┐        │     │
│  │  │  WebRTC  │ │  Direct  │ │  Tauri     │        │     │
│  │  │ Streamer │ │ Callback │ │  Event     │        │     │
│  │  └──────────┘ └──────────┘ └────────────┘        │     │
│  └────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

---

## Implémentation Rust

### 1. Structure du Gestionnaire de Streaming

Créez `src/commands/streaming.rs` :

```rust
use crate::platform::{CameraSystem, PlatformCamera};
use crate::platform::optimizations::{get_optimal_settings, get_photography_format};
use crate::types::{CameraFrame, CameraInitParams};
use crate::webrtc::streaming::WebRTCStreamer;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, AppHandle, Manager};
use tokio::sync::{Mutex as AsyncMutex, RwLock};

/// Mode de streaming
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamingMode {
    /// Streaming WebRTC avec encodage H.264
    WebRTC,
    /// Streaming direct de frames brutes
    Direct,
}

/// Configuration du streaming
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamingConfig {
    pub device_id: String,
    pub mode: StreamingMode,
    /// Utiliser les paramètres optimaux (recommandé)
    #[serde(default = "default_use_optimal")]
    pub use_optimal_settings: bool,
    /// FPS cible (si use_optimal_settings = false)
    pub target_fps: Option<f64>,
    /// Résolution (si use_optimal_settings = false)
    pub resolution: Option<(u32, u32)>,
}

fn default_use_optimal() -> bool {
    true
}

/// État d'une session de streaming
struct StreamingSession {
    camera: Arc<AsyncMutex<PlatformCamera>>,
    mode: StreamingMode,
    webrtc_streamer: Option<Arc<WebRTCStreamer>>,
    is_active: Arc<RwLock<bool>>,
}

lazy_static::lazy_static! {
    static ref STREAMING_SESSIONS: Arc<RwLock<HashMap<String, StreamingSession>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

/// Initialise une caméra avec paramètres optimaux
async fn initialize_optimized_camera(
    device_id: String,
    use_optimal: bool,
    target_fps: Option<f64>,
    resolution: Option<(u32, u32)>,
) -> Result<PlatformCamera, String> {
    let params = if use_optimal {
        // Utilise les paramètres optimisés pour la plateforme
        log::info!("Initializing camera {} with optimal settings", device_id);
        get_optimal_settings()
            .with_device_id(device_id)
    } else {
        // Utilise les paramètres personnalisés
        let mut format = get_photography_format();

        if let Some(fps) = target_fps {
            format = format.with_fps(fps);
        }

        if let Some((width, height)) = resolution {
            format.width = width;
            format.height = height;
        }

        CameraInitParams::new(device_id)
            .with_format(format)
    };

    PlatformCamera::new(params)
        .map_err(|e| format!("Failed to initialize camera: {}", e))
}

/// Démarre le streaming (WebRTC ou Direct)
#[command]
pub async fn start_streaming(
    app: AppHandle,
    config: StreamingConfig,
) -> Result<String, String> {
    log::info!("Starting streaming: {:?}", config);

    let mut sessions = STREAMING_SESSIONS.write().await;

    // Vérifier si une session existe déjà
    if sessions.contains_key(&config.device_id) {
        return Err(format!("Streaming already active for device {}", config.device_id));
    }

    // Initialiser la caméra avec paramètres optimaux
    let mut camera = initialize_optimized_camera(
        config.device_id.clone(),
        config.use_optimal_settings,
        config.target_fps,
        config.resolution,
    ).await?;

    let camera = Arc::new(AsyncMutex::new(camera));
    let is_active = Arc::new(RwLock::new(true));

    match config.mode {
        StreamingMode::WebRTC => {
            start_webrtc_streaming(
                app.clone(),
                camera.clone(),
                config.device_id.clone(),
                &mut sessions,
                is_active.clone(),
            ).await
        }
        StreamingMode::Direct => {
            start_direct_streaming(
                app.clone(),
                camera.clone(),
                config.device_id.clone(),
                &mut sessions,
                is_active.clone(),
            ).await
        }
    }
}

/// Démarre le streaming WebRTC
async fn start_webrtc_streaming(
    app: AppHandle,
    camera: Arc<AsyncMutex<PlatformCamera>>,
    device_id: String,
    sessions: &mut HashMap<String, StreamingSession>,
    is_active: Arc<RwLock<bool>>,
) -> Result<String, String> {
    // Créer le streamer WebRTC
    let streamer = Arc::new(WebRTCStreamer::new());

    // Optionnel: Configurer un callback pour traitement parallèle
    // (ex: détection de visages)
    streamer.set_frame_callback(move |frame| {
        log::debug!("Frame processed: {}x{}", frame.width, frame.height);
        // Ici: traitement personnalisé (ML, détection, etc.)
    });

    // Démarrer le streaming WebRTC
    streamer.start_streaming(device_id.clone()).await?;

    // Enregistrer la session
    sessions.insert(
        device_id.clone(),
        StreamingSession {
            camera,
            mode: StreamingMode::WebRTC,
            webrtc_streamer: Some(streamer),
            is_active,
        },
    );

    Ok(format!("WebRTC streaming started for {}", device_id))
}

/// Démarre le streaming direct (sans WebRTC)
async fn start_direct_streaming(
    app: AppHandle,
    camera: Arc<AsyncMutex<PlatformCamera>>,
    device_id: String,
    sessions: &mut HashMap<String, StreamingSession>,
    is_active: Arc<RwLock<bool>>,
) -> Result<String, String> {
    // Configurer le callback de frame
    let app_clone = app.clone();
    let device_id_clone = device_id.clone();
    let is_active_clone = is_active.clone();

    {
        let mut cam = camera.lock().await;

        // Utiliser la méthode set_frame_callback de la caméra
        // Note: On doit utiliser un closure qui peut être converti en callback
        let callback_app = app_clone.clone();
        let callback_device_id = device_id_clone.clone();

        // On ne peut pas capturer app directement dans le callback car il n'est pas Clone
        // Solution: Utiliser un channel pour envoyer les frames
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CameraFrame>();

        // Configurer le callback sur la caméra plateforme
        // Note: Dépend de la plateforme (Linux, macOS, Windows)
        #[cfg(target_os = "linux")]
        {
            if let crate::platform::PlatformCamera::Linux(ref linux_cam) = &*cam {
                linux_cam.set_frame_callback(move |frame| {
                    let _ = tx.send(frame);
                });
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let crate::platform::PlatformCamera::MacOS(ref macos_cam) = &*cam {
                macos_cam.set_frame_callback(move |frame| {
                    let _ = tx.send(frame);
                });
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let crate::platform::PlatformCamera::Windows(ref windows_cam) = &*cam {
                windows_cam.set_frame_callback(move |frame| {
                    let _ = tx.send(frame);
                });
            }
        }

        // Démarrer le stream
        cam.start_stream()
            .map_err(|e| format!("Failed to start camera stream: {}", e))?;

        // Spawner une tâche pour transmettre les frames au frontend
        let is_active_task = is_active_clone.clone();
        tokio::spawn(async move {
            while *is_active_task.read().await {
                if let Some(frame) = rx.recv().await {
                    // Émettre l'événement Tauri vers le frontend
                    let _ = callback_app.emit_all("camera-frame", frame);
                } else {
                    break;
                }
            }
            log::info!("Direct streaming task ended for {}", callback_device_id);
        });
    }

    // Enregistrer la session
    sessions.insert(
        device_id.clone(),
        StreamingSession {
            camera,
            mode: StreamingMode::Direct,
            webrtc_streamer: None,
            is_active,
        },
    );

    Ok(format!("Direct streaming started for {}", device_id))
}

/// Arrête le streaming
#[command]
pub async fn stop_streaming(device_id: String) -> Result<String, String> {
    log::info!("Stopping streaming for device: {}", device_id);

    let mut sessions = STREAMING_SESSIONS.write().await;

    if let Some(session) = sessions.remove(&device_id) {
        // Marquer comme inactif
        *session.is_active.write().await = false;

        // Arrêter selon le mode
        match session.mode {
            StreamingMode::WebRTC => {
                if let Some(streamer) = session.webrtc_streamer {
                    streamer.stop_streaming().await?;
                }
            }
            StreamingMode::Direct => {
                let mut cam = session.camera.lock().await;

                // Clear callback et arrêter le stream
                #[cfg(target_os = "linux")]
                {
                    if let crate::platform::PlatformCamera::Linux(ref linux_cam) = &*cam {
                        linux_cam.clear_frame_callback();
                    }
                }

                #[cfg(target_os = "macos")]
                {
                    if let crate::platform::PlatformCamera::MacOS(ref macos_cam) = &*cam {
                        macos_cam.clear_frame_callback();
                    }
                }

                #[cfg(target_os = "windows")]
                {
                    if let crate::platform::PlatformCamera::Windows(ref windows_cam) = &*cam {
                        windows_cam.clear_frame_callback();
                    }
                }

                cam.stop_stream()
                    .map_err(|e| format!("Failed to stop camera stream: {}", e))?;
            }
        }

        Ok(format!("Streaming stopped for {}", device_id))
    } else {
        Err(format!("No active streaming session for device {}", device_id))
    }
}

/// Obtenir l'état du streaming
#[command]
pub async fn get_streaming_status(device_id: String) -> Result<StreamingStatus, String> {
    let sessions = STREAMING_SESSIONS.read().await;

    if let Some(session) = sessions.get(&device_id) {
        let is_active = *session.is_active.read().await;
        Ok(StreamingStatus {
            device_id,
            is_active,
            mode: session.mode.clone(),
        })
    } else {
        Ok(StreamingStatus {
            device_id,
            is_active: false,
            mode: StreamingMode::Direct, // Valeur par défaut
        })
    }
}

/// Liste toutes les sessions actives
#[command]
pub async fn list_active_streams() -> Result<Vec<StreamingStatus>, String> {
    let sessions = STREAMING_SESSIONS.read().await;
    let mut statuses = Vec::new();

    for (device_id, session) in sessions.iter() {
        let is_active = *session.is_active.read().await;
        statuses.push(StreamingStatus {
            device_id: device_id.clone(),
            is_active,
            mode: session.mode.clone(),
        });
    }

    Ok(statuses)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamingStatus {
    pub device_id: String,
    pub is_active: bool,
    pub mode: StreamingMode,
}
```

### 2. Enregistrement dans lib.rs

Modifiez `src/lib.rs` pour inclure le nouveau module :

```rust
// Ajoutez au début du fichier
pub mod commands {
    pub mod streaming;
    // ... autres modules
}

// Dans la fonction builder
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Commandes existantes...
            commands::streaming::start_streaming,
            commands::streaming::stop_streaming,
            commands::streaming::get_streaming_status,
            commands::streaming::list_active_streams,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Commandes Tauri

### Commandes Disponibles

| Commande | Description | Paramètres | Retour |
|----------|-------------|------------|--------|
| `start_streaming` | Démarre le streaming | `StreamingConfig` | `String` (status) |
| `stop_streaming` | Arrête le streaming | `device_id: String` | `String` (status) |
| `get_streaming_status` | Obtient l'état | `device_id: String` | `StreamingStatus` |
| `list_active_streams` | Liste les sessions actives | - | `Vec<StreamingStatus>` |

---

## Intégration Frontend

### 1. Types TypeScript

Créez `src/types/streaming.ts` :

```typescript
export type StreamingMode = 'webrtc' | 'direct';

export interface StreamingConfig {
  device_id: string;
  mode: StreamingMode;
  use_optimal_settings?: boolean; // true par défaut
  target_fps?: number;
  resolution?: [number, number];
}

export interface StreamingStatus {
  device_id: string;
  is_active: boolean;
  mode: StreamingMode;
}

export interface CameraFrame {
  id: string;
  data: number[];
  width: number;
  height: number;
  device_id: string;
  format?: string;
  timestamp?: string;
  size_bytes: number;
}
```

### 2. Service de Streaming

Créez `src/services/streamingService.ts` :

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { StreamingConfig, StreamingStatus, CameraFrame } from '../types/streaming';

export class StreamingService {
  private frameListener: UnlistenFn | null = null;

  /**
   * Démarre le streaming
   */
  async start(config: StreamingConfig): Promise<string> {
    try {
      const result = await invoke<string>('start_streaming', { config });
      console.log('Streaming started:', result);
      return result;
    } catch (error) {
      console.error('Failed to start streaming:', error);
      throw error;
    }
  }

  /**
   * Arrête le streaming
   */
  async stop(deviceId: string): Promise<string> {
    try {
      // Nettoyer le listener de frames
      if (this.frameListener) {
        this.frameListener();
        this.frameListener = null;
      }

      const result = await invoke<string>('stop_streaming', { deviceId });
      console.log('Streaming stopped:', result);
      return result;
    } catch (error) {
      console.error('Failed to stop streaming:', error);
      throw error;
    }
  }

  /**
   * Obtenir l'état du streaming
   */
  async getStatus(deviceId: string): Promise<StreamingStatus> {
    return await invoke<StreamingStatus>('get_streaming_status', { deviceId });
  }

  /**
   * Lister les sessions actives
   */
  async listActive(): Promise<StreamingStatus[]> {
    return await invoke<StreamingStatus[]>('list_active_streams');
  }

  /**
   * Écouter les frames en mode Direct
   */
  async listenToFrames(callback: (frame: CameraFrame) => void): Promise<void> {
    // Nettoyer l'ancien listener si existe
    if (this.frameListener) {
      this.frameListener();
    }

    // Créer un nouveau listener
    this.frameListener = await listen<CameraFrame>('camera-frame', (event) => {
      callback(event.payload);
    });
  }

  /**
   * Arrêter l'écoute des frames
   */
  stopListeningFrames(): void {
    if (this.frameListener) {
      this.frameListener();
      this.frameListener = null;
    }
  }
}
```

### 3. Composant React de Streaming

Créez `src/components/StreamingPlayer.tsx` :

```typescript
import React, { useEffect, useRef, useState } from 'react';
import { StreamingService } from '../services/streamingService';
import { StreamingConfig, CameraFrame } from '../types/streaming';

interface StreamingPlayerProps {
  deviceId: string;
  mode: 'webrtc' | 'direct';
  onError?: (error: string) => void;
}

export const StreamingPlayer: React.FC<StreamingPlayerProps> = ({
  deviceId,
  mode,
  onError,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [fps, setFps] = useState(0);
  const streamingService = useRef(new StreamingService());

  // Compteur de FPS
  useEffect(() => {
    let frameCount = 0;
    const interval = setInterval(() => {
      setFps(frameCount);
      frameCount = 0;
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  // Démarrer le streaming
  const startStreaming = async () => {
    try {
      const config: StreamingConfig = {
        device_id: deviceId,
        mode: mode,
        use_optimal_settings: true, // Utiliser les paramètres optimaux
      };

      await streamingService.current.start(config);
      setIsStreaming(true);

      if (mode === 'direct') {
        // Écouter les frames en mode Direct
        await streamingService.current.listenToFrames((frame) => {
          drawFrameOnCanvas(frame);
        });
      }
      // Mode WebRTC géré séparément (voir section WebRTC)
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      onError?.(errorMsg);
    }
  };

  // Arrêter le streaming
  const stopStreaming = async () => {
    try {
      await streamingService.current.stop(deviceId);
      setIsStreaming(false);
      streamingService.current.stopListeningFrames();
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      onError?.(errorMsg);
    }
  };

  // Dessiner une frame sur le canvas (mode Direct)
  const drawFrameOnCanvas = (frame: CameraFrame) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Ajuster la taille du canvas
    canvas.width = frame.width;
    canvas.height = frame.height;

    // Créer ImageData depuis les données RGB
    const imageData = ctx.createImageData(frame.width, frame.height);
    const data = imageData.data;

    // Convertir RGB vers RGBA
    for (let i = 0; i < frame.data.length; i += 3) {
      const j = (i / 3) * 4;
      data[j] = frame.data[i];         // R
      data[j + 1] = frame.data[i + 1]; // G
      data[j + 2] = frame.data[i + 2]; // B
      data[j + 3] = 255;               // A
    }

    ctx.putImageData(imageData, 0, 0);
  };

  // Cleanup au démontage
  useEffect(() => {
    return () => {
      if (isStreaming) {
        stopStreaming();
      }
    };
  }, []);

  return (
    <div className="streaming-player">
      <div className="controls">
        <button onClick={startStreaming} disabled={isStreaming}>
          Start Streaming
        </button>
        <button onClick={stopStreaming} disabled={!isStreaming}>
          Stop Streaming
        </button>
        <span>FPS: {fps}</span>
        <span>Mode: {mode.toUpperCase()}</span>
      </div>

      <div className="video-container">
        {mode === 'direct' ? (
          <canvas ref={canvasRef} />
        ) : (
          <video ref={videoRef} autoPlay playsInline />
        )}
      </div>
    </div>
  );
};
```

---

## Exemples Complets

### Exemple 1 : Streaming Direct Simple

```typescript
import React from 'react';
import { StreamingPlayer } from './components/StreamingPlayer';

function App() {
  return (
    <div className="App">
      <h1>CrabCamera - Streaming Direct</h1>
      <StreamingPlayer
        deviceId="0"
        mode="direct"
        onError={(error) => console.error('Streaming error:', error)}
      />
    </div>
  );
}

export default App;
```

### Exemple 2 : Streaming WebRTC

```typescript
import React, { useEffect, useRef, useState } from 'react';
import { StreamingService } from '../services/streamingService';

export const WebRTCPlayer: React.FC<{ deviceId: string }> = ({ deviceId }) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [pc, setPc] = useState<RTCPeerConnection | null>(null);
  const streamingService = useRef(new StreamingService());

  const startWebRTC = async () => {
    // Démarrer le streaming côté Rust
    await streamingService.current.start({
      device_id: deviceId,
      mode: 'webrtc',
      use_optimal_settings: true,
    });

    // Créer la connexion WebRTC (code existant de WEBRTC_STREAMING_GUIDE.md)
    const peerConnection = new RTCPeerConnection({
      iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
    });

    // Configuration WebRTC...
    // (Voir WEBRTC_STREAMING_GUIDE.md pour le code complet)

    setPc(peerConnection);
  };

  const stopWebRTC = async () => {
    if (pc) {
      pc.close();
      setPc(null);
    }
    await streamingService.current.stop(deviceId);
  };

  return (
    <div>
      <button onClick={startWebRTC}>Start WebRTC</button>
      <button onClick={stopWebRTC}>Stop WebRTC</button>
      <video ref={videoRef} autoPlay playsInline />
    </div>
  );
};
```

### Exemple 3 : Commutation entre Modes

```typescript
import React, { useState } from 'react';
import { StreamingPlayer } from './components/StreamingPlayer';

type StreamMode = 'direct' | 'webrtc';

export const ModeSwitch: React.FC = () => {
  const [mode, setMode] = useState<StreamMode>('direct');
  const [isStreaming, setIsStreaming] = useState(false);

  const switchMode = async (newMode: StreamMode) => {
    // Arrêter le streaming actuel si actif
    if (isStreaming) {
      // Le composant StreamingPlayer gère l'arrêt
      setIsStreaming(false);
      // Attendre un peu pour que le streaming s'arrête proprement
      await new Promise(resolve => setTimeout(resolve, 500));
    }

    setMode(newMode);
  };

  return (
    <div>
      <div className="mode-selector">
        <button onClick={() => switchMode('direct')} disabled={mode === 'direct'}>
          Mode Direct
        </button>
        <button onClick={() => switchMode('webrtc')} disabled={mode === 'webrtc'}>
          Mode WebRTC
        </button>
      </div>

      <StreamingPlayer
        deviceId="0"
        mode={mode}
        onError={(error) => console.error(error)}
      />
    </div>
  );
};
```

---

## Optimisations et Bonnes Pratiques

### 1. Mutualisation du Code

La fonction `initialize_optimized_camera()` centralise la logique d'initialisation :

```rust
// Utilisée automatiquement par start_streaming()
// Applique get_optimal_settings() si use_optimal_settings = true
// Sinon utilise get_photography_format() avec paramètres custom
```

### 2. Gestion des Ressources

```rust
// Le gestionnaire STREAMING_SESSIONS garantit :
// - Une seule session par device_id
// - Nettoyage automatique à l'arrêt
// - Pas de fuite mémoire
```

### 3. Performance Frontend

```typescript
// Mode Direct : Canvas pour le rendu rapide
// Mode WebRTC : Video natif avec décodage hardware

// Limiter les re-renders
const drawFrameOnCanvas = useCallback((frame: CameraFrame) => {
  // Logique de dessin
}, []);
```

### 4. Gestion des Erreurs

```rust
// Toutes les commandes retournent Result<T, String>
// Les erreurs sont propagées au frontend
// Le frontend peut afficher des messages utilisateur
```

---

## Dépendances Requises

### Cargo.toml

```toml
[dependencies]
crabcamera = { path = ".", features = ["webrtc"] }
tauri = "2.0"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
lazy_static = "1.4"
```

### package.json (Frontend)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  }
}
```

---

## Résumé

| Aspect | Implémentation |
|--------|----------------|
| **Initialisation** | `get_optimal_settings()` + `get_photography_format()` |
| **Modes** | WebRTC (encodé) et Direct (brut) |
| **Rust** | 90% de la logique (gestion caméra, streaming, callbacks) |
| **Frontend** | 10% (affichage, contrôles UI) |
| **Mutualisation** | `StreamingSession` unifie les deux modes |
| **Performance** | Callbacks natifs, zero-copy, hardware decode |

Cette architecture maximise la logique côté Rust tout en gardant le frontend léger et réactif.
