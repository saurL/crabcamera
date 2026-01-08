# CrabCameraManager - Guide d'Utilisation Complet

Ce guide explique comment utiliser `CrabCameraManager` pour gérer les caméras, configurer les streams, et intégrer WebRTC dans votre application Tauri.

## Table des Matières

1. [Initialisation](#initialisation)
2. [Découverte et Configuration des Devices](#découverte-et-configuration-des-devices)
3. [Streaming Direct (Sans WebRTC)](#streaming-direct-sans-webrtc)
4. [Streaming WebRTC avec Callback](#streaming-webrtc-avec-callback)
5. [Connexion Frontend au Stream WebRTC](#connexion-frontend-au-stream-webrtc)
6. [Exemples Complets](#exemples-complets)

---

## Initialisation

### 1. Setup dans votre application Tauri

```rust
// src-tauri/src/main.rs
use crabcamera;

fn main() {
    tauri::Builder::default()
        .plugin(crabcamera::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. Accès au CrabCameraManager depuis Rust

```rust
use crabcamera::CrabCameraManager;
use tauri::{AppHandle, Runtime, Manager};

// Dans une commande Tauri ou un état
#[tauri::command]
async fn my_camera_command<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    // Récupérer le plugin
    let camera_plugin = app.crabcamera();

    // Utiliser le manager
    let cameras = camera_plugin.get_available_cameras().await?;

    Ok(())
}
```

---

## Découverte et Configuration des Devices

### 1. Lister les Caméras Disponibles

```rust
use crabcamera::CrabCameraManager;
use tauri::AppHandle;

async fn discover_cameras<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let manager = app.crabcamera();;

    // Initialiser le système de caméra
    manager.initialize_camera_system().await?;

    // Récupérer toutes les caméras disponibles
    let cameras = manager.get_available_cameras().await?;

    println!("Caméras détectées:");
    for camera in cameras {
        println!("  - ID: {}", camera.id);
        println!("    Nom: {}", camera.name);
        println!("    Description: {}", camera.description);
        println!("    Disponible: {}", camera.is_available);

        // Afficher les formats supportés
        for format in camera.formats {
            println!("      Format: {}x{} @ {} fps",
                format.width, format.height, format.frame_rate);
        }
    }

    Ok(())
}
```

### 2. Vérifier la Disponibilité d'une Caméra

```rust
async fn check_camera<R: Runtime>(
    app: AppHandle<R>,
    device_id: String
) -> Result<bool, String> {
    let manager = app.crabcamera();;

    // Vérifier si la caméra est disponible
    let is_available = manager.check_camera_availability(device_id.clone()).await?;

    if is_available {
        println!("Caméra {} est disponible", device_id);

        // Récupérer les formats supportés
        let formats = manager.get_camera_formats(device_id).await?;
        println!("Formats supportés: {:?}", formats);
    } else {
        println!("Caméra {} n'est pas disponible", device_id);
    }

    Ok(is_available)
}
```

### 3. Configurer une Caméra

```rust
use crabcamera::types::{CameraControls, WhiteBalance, FocusMode};

async fn configure_camera<R: Runtime>(
    app: AppHandle<R>,
    device_id: String
) -> Result<(), String> {
    let manager = app.crabcamera();;

    // Créer une configuration personnalisée
    let mut controls = CameraControls::default();

    // Configurer les paramètres
    controls.brightness = Some(50);
    controls.contrast = Some(50);
    controls.saturation = Some(50);
    controls.sharpness = Some(50);

    // Configuration de l'exposition
    controls.exposure_mode = Some("manual".to_string());
    controls.exposure_time = Some(0.016); // 16ms
    controls.iso = Some(400);

    // Configuration du focus
    controls.focus_mode = Some(FocusMode::Manual);
    controls.focus_distance = Some(0.5); // Distance 0.0-1.0

    // Balance des blancs
    controls.white_balance_mode = Some(WhiteBalance::Manual(5500)); // 5500K

    // Appliquer les contrôles
    manager.set_camera_controls(device_id.clone(), controls).await?;

    println!("Configuration de la caméra {} appliquée", device_id);

    // Vérifier les capacités de la caméra
    let capabilities = manager.test_camera_capabilities(device_id.clone()).await?;
    println!("Capacités de la caméra:");
    println!("  - Auto focus: {}", capabilities.supports_auto_focus);
    println!("  - Manual focus: {}", capabilities.supports_manual_focus);
    println!("  - HDR: {}", capabilities.supports_hdr);
    println!("  - Burst mode: {}", capabilities.supports_burst_mode);

    Ok(())
}
```

---

## Streaming Direct (Sans WebRTC)

### Configuration et Lancement du Stream Direct

Le streaming direct émet les frames via des événements Tauri que le frontend peut écouter.

```rust
use crabcamera::commands::streaming::StreamConfig;
use crabcamera::types::CameraFormat;

async fn start_direct_camera_stream<R: Runtime>(
    app: AppHandle<R>,
    device_id: String
) -> Result<(), String> {
    let manager = app.crabcamera();;

    // Option 1: Utiliser les paramètres optimaux (recommandé)
    let config = StreamConfig {
        device_id: device_id.clone(),
        use_optimal_settings: true,
        format: None,
    };

    // Option 2: Configuration personnalisée
    let custom_config = StreamConfig {
        device_id: device_id.clone(),
        use_optimal_settings: false,
        format: Some(CameraFormat::new(1920, 1080, 30.0)),
    };
    // Définir un callback pour chaque frame capturée
    let callback = Box::new(move |frame: CameraFrame| {
        println!("Frame capturée: {}x{} @ {}",
            frame.width, frame.height, frame.timestamp);
        // Traitement personnalisé de la frame
    });
    // Démarrer le streaming direct
    let result = manager.start_direct_streaming(config,callback).await?;
    println!("{}", result);

    // Les frames seront automatiquement émises via l'événement "camera-frame"
    // Le frontend peut les écouter avec listen('camera-frame', ...)

    Ok(())
}

async fn stop_direct_stream<R: Runtime>(
    app: AppHandle<R>,
    device_id: String
) -> Result<(), String> {
    let manager = app.crabcamera();;

    // Arrêter le streaming
    manager.stop_direct_streaming(device_id.clone()).await?;

    println!("Stream arrêté pour device {}", device_id);

    Ok(())
}

async fn check_stream_status<R: Runtime>(
    app: AppHandle<R>,
    device_id: String
) -> Result<(), String> {
    let manager = app.crabcamera();;

    // Vérifier le statut du stream
    let status = manager.get_streaming_status(device_id).await?;

    println!("Stream actif: {}", status.is_active);

    // Lister tous les streams actifs
    let active_streams = manager.list_active_streams().await?;
    println!("Nombre de streams actifs: {}", active_streams.len());

    Ok(())
}
```

---

## Streaming WebRTC avec Callback

### 1. Démarrer un Stream WebRTC avec Callback pour Chaque Frame

```rust
use crabcamera::webrtc::streaming::StreamConfig as WebRTCStreamConfig;
use crabcamera::webrtc::StreamMode;
use crabcamera::types::CameraFrame;
use std::sync::Arc;
use tokio::sync::Mutex;

async fn start_webrtc_stream_with_callback<R: Runtime>(
    app: AppHandle<R>,
    device_id: String,
    stream_id: String
) -> Result<(), String> {
    let manager = app.crabcamera();;

    // Configuration WebRTC personnalisée
    let webrtc_config = Some(WebRTCStreamConfig {
        width: 1920,
        height: 1080,
        fps: 30.0,
        bitrate: 2_000_000, // 2 Mbps
        codec: "h264".to_string(),
    });

let callback = Box::new(move |frame: &CameraFrame| {
        println!("Frame WebRTC capturée: {}x{} @ {}",
            frame.width, frame.height, frame.timestamp);

        // Traitement personnalisé de la frame
        // Par exemple: détection de visage, analyse de qualité, etc.
    });
    // Démarrer le stream WebRTC
    manager.start_webrtc_streaming(
        device_id.clone(),
        stream_id.clone(),
        webrtc_config,
        Some(StreamMode::RealCamera),
        callback
    ).await?;

    println!("WebRTC stream {} démarré", stream_id);

    // Note: Le callback est géré différemment pour WebRTC
    // Les frames WebRTC sont encodées en H.264 et envoyées via RTP
    // Pour traiter les frames raw, utilisez set_frame_callback sur le streamer

    Ok(())
}
```

## Connexion Frontend au Stream WebRTC

### 1. Configuration de la Connexion Peer-to-Peer (JavaScript)

```javascript
// frontend/src/webrtc.js
import { invoke } from "@tauri-apps/api/core";

class WebRTCClient {
  constructor(peerId, streamId) {
    this.peerId = peerId;
    this.streamId = streamId;
    this.peerConnection = null;
  }

  async initialize() {
    // 1. Créer la PeerConnection côté navigateur
    this.peerConnection = new RTCPeerConnection({
      iceServers: [
        { urls: "stun:stun.l.google.com:19302" },
        { urls: "stun:stun1.l.google.com:19302" },
      ],
    });

    // 2. Créer la connexion peer côté Rust
    await invoke("plugin:crabcamera|create_peer_connection", {
      peerId: this.peerId,
      config: {
        ice_servers: [
          "stun:stun.l.google.com:19302",
          "stun:stun1.l.google.com:19302",
        ],
      },
    });

    // 3. Ajouter les transceivers vidéo
    // Tableau vide = configuration par défaut
    await invoke("plugin:crabcamera|add_video_transceivers", {
      peerId: this.peerId,
      layers: [],
    });

    // 4. Associer le stream à la peer connection
    await invoke("plugin:crabcamera|associate_stream_with_peer", {
      streamId: this.streamId,
      peerId: this.peerId,
    });

    // 5. Créer l'offre SDP côté Rust
    const offer = await invoke("plugin:crabcamera|create_webrtc_offer", {
      peerId: this.peerId,
    });

    // 6. Définir l'offre distante dans le navigateur
    await this.peerConnection.setRemoteDescription({
      type: offer.sdp_type,
      sdp: offer.sdp,
    });

    // 7. Créer la réponse SDP côté navigateur
    const answer = await this.peerConnection.createAnswer();
    await this.peerConnection.setLocalDescription(answer);

    // 8. Envoyer la réponse au backend Rust
    await invoke("plugin:crabcamera|set_remote_description", {
      peerId: this.peerId,
      description: {
        sdp_type: "answer",
        sdp: answer.sdp,
      },
    });

    // 9. Gérer les candidats ICE du navigateur
    this.peerConnection.onicecandidate = async (event) => {
      if (event.candidate) {
        await invoke("plugin:crabcamera|add_ice_candidate", {
          peerId: this.peerId,
          candidate: {
            candidate: event.candidate.candidate,
            sdp_mid: event.candidate.sdpMid || "",
            sdp_m_line_index: event.candidate.sdpMLineIndex || 0,
          },
        });
      }
    };

    // 10. Gérer la réception du stream vidéo
    this.peerConnection.ontrack = (event) => {
      console.log("Track vidéo reçu:", event.track.kind);
      const video = document.getElementById("webrtc-video");
      if (video) {
        video.srcObject = event.streams[0];
        video.play();
      }
    };

    console.log("✅ WebRTC connection established");
  }

  async disconnect() {
    if (this.peerConnection) {
      this.peerConnection.close();
      this.peerConnection = null;
    }

    await invoke("plugin:crabcamera|close_peer_connection", {
      peerId: this.peerId,
    });
  }

  async getStatus() {
    return await invoke("plugin:crabcamera|get_peer_connection_status", {
      peerId: this.peerId,
    });
  }
}

// Fonction principale pour démarrer le streaming
async function startWebRTCStream() {
  const deviceId = "0";
  const streamId = "camera-stream-" + Date.now();
  const peerId = "browser-peer-" + Date.now();

  try {
    // 1. Initialiser le système de caméra
    await invoke("plugin:crabcamera|initialize_camera_system");

    // 2. Démarrer le stream WebRTC côté Rust
    const result = await invoke("plugin:crabcamera|start_webrtc_stream", {
      deviceId: deviceId,
      streamId: streamId,
      config: {
        width: 1280,
        height: 720,
        fps: 30.0,
        bitrate: 2000000,
        codec: "h264",
      },
      mode: "RealCamera",
    });
    console.log("Stream démarré:", result);

    // 3. Établir la connexion WebRTC
    const client = new WebRTCClient(peerId, streamId);
    await client.initialize();

    console.log("📡 Streaming WebRTC actif!");
    return { client, streamId };
  } catch (error) {
    console.error("❌ Erreur:", error);
    throw error;
  }
}

// Arrêter le streaming
async function stopWebRTCStream(client, streamId) {
  try {
    if (client) {
      await client.disconnect();
    }

    await invoke("plugin:crabcamera|stop_webrtc_stream", {
      streamId: streamId,
    });

    console.log("✅ Stream arrêté");
  } catch (error) {
    console.error("❌ Erreur:", error);
  }
}

// Fonctions utilitaires
async function getStreamStatus(streamId) {
  return await invoke("plugin:crabcamera|get_webrtc_stream_status", {
    streamId: streamId,
  });
}

async function listActiveStreams() {
  return await invoke("plugin:crabcamera|list_webrtc_streams");
}

async function pauseStream(streamId) {
  return await invoke("plugin:crabcamera|pause_webrtc_stream", {
    streamId: streamId,
  });
}

async function resumeStream(streamId) {
  return await invoke("plugin:crabcamera|resume_webrtc_stream", {
    streamId: streamId,
  });
}

export {
  WebRTCClient,
  startWebRTCStream,
  stopWebRTCStream,
  getStreamStatus,
  listActiveStreams,
  pauseStream,
  resumeStream
};
```

### 2. HTML pour Afficher le Stream

```html
<!DOCTYPE html>
<html>
  <head>
    <title>CrabCamera WebRTC Stream</title>
    <style>
      body {
        font-family: Arial, sans-serif;
        margin: 20px;
        background: #1a1a1a;
        color: #fff;
      }

      #webrtc-video {
        width: 100%;
        max-width: 1280px;
        height: auto;
        background: #000;
        border-radius: 8px;
      }

      .controls {
        margin: 20px 0;
      }

      button {
        padding: 12px 24px;
        margin: 5px;
        font-size: 16px;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        background: #007bff;
        color: white;
        transition: background 0.3s;
      }

      button:hover {
        background: #0056b3;
      }

      button:disabled {
        background: #666;
        cursor: not-allowed;
      }

      #stats {
        margin-top: 20px;
        padding: 15px;
        background: #2a2a2a;
        border-radius: 8px;
        font-family: monospace;
      }
    </style>
  </head>
  <body>
    <h1>🦀 CrabCamera WebRTC Stream</h1>

    <div class="controls">
      <button id="start-btn">▶️ Démarrer Stream</button>
      <button id="stop-btn" disabled>⏹️ Arrêter Stream</button>
      <button id="pause-btn" disabled>⏸️ Pause</button>
      <button id="resume-btn" disabled>▶️ Reprendre</button>
      <button id="stats-btn" disabled>📊 Statistiques</button>
    </div>

    <video id="webrtc-video" autoplay playsinline muted></video>

    <div id="stats"></div>

    <script type="module">
      import {
        startWebRTCStream,
        stopWebRTCStream,
        pauseStream,
        resumeStream,
        getStreamStatus
      } from "./webrtc.js";

      let client = null;
      let streamId = null;

      const startBtn = document.getElementById("start-btn");
      const stopBtn = document.getElementById("stop-btn");
      const pauseBtn = document.getElementById("pause-btn");
      const resumeBtn = document.getElementById("resume-btn");
      const statsBtn = document.getElementById("stats-btn");
      const statsDiv = document.getElementById("stats");

      startBtn.addEventListener("click", async () => {
        try {
          startBtn.disabled = true;
          const result = await startWebRTCStream();
          client = result.client;
          streamId = result.streamId;

          stopBtn.disabled = false;
          pauseBtn.disabled = false;
          statsBtn.disabled = false;
        } catch (error) {
          startBtn.disabled = false;
          alert("Erreur: " + error);
        }
      });

      stopBtn.addEventListener("click", async () => {
        try {
          stopBtn.disabled = true;
          await stopWebRTCStream(client, streamId);

          client = null;
          streamId = null;
          startBtn.disabled = false;
          pauseBtn.disabled = true;
          resumeBtn.disabled = true;
          statsBtn.disabled = true;
          statsDiv.textContent = "";
        } catch (error) {
          alert("Erreur: " + error);
        }
      });

      pauseBtn.addEventListener("click", async () => {
        if (streamId) {
          await pauseStream(streamId);
          pauseBtn.disabled = true;
          resumeBtn.disabled = false;
        }
      });

      resumeBtn.addEventListener("click", async () => {
        if (streamId) {
          await resumeStream(streamId);
          resumeBtn.disabled = true;
          pauseBtn.disabled = false;
        }
      });

      statsBtn.addEventListener("click", async () => {
        if (streamId) {
          const stats = await getStreamStatus(streamId);
          statsDiv.innerHTML = `
            <strong>Statistiques du Stream:</strong><br>
            Stream ID: ${stats.stream_id}<br>
            Résolution: ${stats.width}x${stats.height}<br>
            FPS: ${stats.fps}<br>
            Bitrate: ${stats.bitrate} bps<br>
            Codec: ${stats.codec}<br>
            Frames encodées: ${stats.frames_encoded}<br>
            Bytes envoyés: ${stats.bytes_sent}
          `;
        }
      });
    </script>
  </body>
</html>
```

### 3. Liste des Commandes WebRTC Disponibles

Voici toutes les commandes WebRTC accessibles depuis le frontend:

```javascript
// Gestion des streams
"plugin:crabcamera|start_webrtc_stream"
"plugin:crabcamera|stop_webrtc_stream"
"plugin:crabcamera|get_webrtc_stream_status"
"plugin:crabcamera|update_webrtc_config"
"plugin:crabcamera|list_webrtc_streams"
"plugin:crabcamera|pause_webrtc_stream"
"plugin:crabcamera|resume_webrtc_stream"
"plugin:crabcamera|set_webrtc_stream_bitrate"

// Gestion des peer connections
"plugin:crabcamera|create_peer_connection"
"plugin:crabcamera|create_webrtc_offer"
"plugin:crabcamera|create_webrtc_answer"
"plugin:crabcamera|set_remote_description"
"plugin:crabcamera|add_ice_candidate"
"plugin:crabcamera|get_local_ice_candidates"
"plugin:crabcamera|close_peer_connection"
"plugin:crabcamera|get_peer_connection_status"
"plugin:crabcamera|list_peer_connections"

// Gestion des transceivers et data channels
"plugin:crabcamera|add_video_transceivers"
"plugin:crabcamera|create_data_channel"
"plugin:crabcamera|send_data_channel_message"

// Statut système
"plugin:crabcamera|get_webrtc_system_status"
```

---

## Notes Importantes

- **Thread Safety**: Toutes les méthodes du `CrabCameraManager` sont async et thread-safe
- **Performance**: Les callbacks de frame s'exécutent dans des threads séparés
- **Ressources**: Pensez à toujours arrêter les streams pour libérer les ressources
- **WebRTC**: Nécessite une négociation SDP/ICE complète entre Rust et JavaScript

Pour plus d'informations, consultez:

- Documentation de l'API: `src/commands/`
- Tests: `src/tests/`
- Exemples WebRTC: `src/webrtc/`
