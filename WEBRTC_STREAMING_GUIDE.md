# 📡 Guide WebRTC Streaming - CrabCamera

Guide complet pour implémenter le streaming WebRTC avec CrabCamera dans votre application Tauri.

## 🎯 Vue d'ensemble

Le streaming WebRTC de CrabCamera permet de diffuser en temps réel le flux vidéo de la caméra vers un navigateur web avec:
- **Encodage H.264** hardware-accelerated
- **Négociation SDP/ICE** automatique
- **Support simulcast** multi-qualités
- **Data channels** pour la communication bidirectionnelle
- **Callbacks personnalisés** pour le traitement des frames

---

## 🚀 Démarrage Rapide

### Étape 1: Démarrer le Stream (Backend Rust)

```rust
use crabcamera::CrabCameraExt;
use tauri::{AppHandle, Runtime};

#[tauri::command]
async fn start_camera_stream<R: Runtime>(
    app: AppHandle<R>,
    device_id: String,
) -> Result<String, String> {
    let manager = app.crabcamera();

    // Initialiser le système
    manager.initialize_camera_system().await?;

    // Démarrer le stream WebRTC
    manager.start_webrtc_stream(
        device_id,
        "main-stream".to_string(),
        Some(crate::webrtc::streaming::StreamConfig {
            width: 1280,
            height: 720,
            fps: 30.0,
            bitrate: 2_000_000,
            codec: "h264".to_string(),
        }),
        Some(crate::webrtc::StreamMode::RealCamera),
    ).await
}
```

### Étape 2: Connexion WebRTC (Frontend JavaScript)

```javascript
import { invoke } from "@tauri-apps/api/core";

async function connectToStream() {
    const deviceId = "0";
    const streamId = "main-stream";
    const peerId = "browser-" + Date.now();

    // 1. Démarrer le stream
    await invoke("start_camera_stream", { deviceId });

    // 2. Créer la peer connection
    await invoke("plugin:crabcamera|create_peer_connection", {
        peerId,
        config: { ice_servers: ["stun:stun.l.google.com:19302"] }
    });

    // 3. Ajouter transceivers
    await invoke("plugin:crabcamera|add_video_transceivers", {
        peerId,
        layers: []
    });

    // 4. Associer stream et peer
    await invoke("plugin:crabcamera|associate_stream_with_peer", {
        streamId,
        peerId
    });

    // 5. Créer PeerConnection navigateur
    const pc = new RTCPeerConnection({
        iceServers: [{ urls: "stun:stun.l.google.com:19302" }]
    });

    // 6. Obtenir l'offre SDP
    const offer = await invoke("plugin:crabcamera|create_webrtc_offer", { peerId });

    // 7. Négociation SDP
    await pc.setRemoteDescription({ type: offer.sdp_type, sdp: offer.sdp });
    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);

    // 8. Envoyer la réponse
    await invoke("plugin:crabcamera|set_remote_description", {
        peerId,
        description: { sdp_type: "answer", sdp: answer.sdp }
    });

    // 9. Gérer ICE candidates
    pc.onicecandidate = async (e) => {
        if (e.candidate) {
            await invoke("plugin:crabcamera|add_ice_candidate", {
                peerId,
                candidate: {
                    candidate: e.candidate.candidate,
                    sdp_mid: e.candidate.sdpMid || "",
                    sdp_m_line_index: e.candidate.sdpMLineIndex || 0
                }
            });
        }
    };

    // 10. Afficher le stream
    pc.ontrack = (e) => {
        document.getElementById("video").srcObject = e.streams[0];
    };
}
```

---

## 📋 Commandes WebRTC Disponibles

### Gestion des Streams

#### `start_webrtc_stream`
Démarre un nouveau stream WebRTC.

```javascript
await invoke("plugin:crabcamera|start_webrtc_stream", {
    deviceId: "0",
    streamId: "my-stream",
    config: {
        width: 1920,
        height: 1080,
        fps: 30.0,
        bitrate: 4000000,
        codec: "h264"
    },
    mode: "RealCamera"  // ou "TestPattern"
});
```

**Paramètres:**
- `deviceId`: ID de la caméra (obtenu via `get_available_cameras`)
- `streamId`: Identifiant unique pour ce stream
- `config`: Configuration optionnelle du stream
- `mode`: Mode de streaming (`RealCamera` ou `TestPattern`)

#### `stop_webrtc_stream`
Arrête un stream actif.

```javascript
await invoke("plugin:crabcamera|stop_webrtc_stream", {
    streamId: "my-stream"
});
```

#### `get_webrtc_stream_status`
Obtient les statistiques d'un stream.

```javascript
const stats = await invoke("plugin:crabcamera|get_webrtc_stream_status", {
    streamId: "my-stream"
});

console.log(`FPS: ${stats.fps}, Bitrate: ${stats.bitrate}`);
```

**Retour:**
```typescript
{
    stream_id: string,
    width: number,
    height: number,
    fps: number,
    bitrate: number,
    codec: string,
    frames_encoded: number,
    bytes_sent: number,
    is_active: boolean
}
```

#### `list_webrtc_streams`
Liste tous les streams actifs.

```javascript
const streams = await invoke("plugin:crabcamera|list_webrtc_streams");
streams.forEach(s => console.log(`Stream ${s.stream_id}: ${s.width}x${s.height}`));
```

#### `pause_webrtc_stream` / `resume_webrtc_stream`
Met en pause ou reprend un stream.

```javascript
await invoke("plugin:crabcamera|pause_webrtc_stream", { streamId: "my-stream" });
await invoke("plugin:crabcamera|resume_webrtc_stream", { streamId: "my-stream" });
```

#### `set_webrtc_stream_bitrate`
Modifie le bitrate dynamiquement.

```javascript
await invoke("plugin:crabcamera|set_webrtc_stream_bitrate", {
    streamId: "my-stream",
    bitrate: 1000000  // 1 Mbps
});
```

---

### Gestion des Peer Connections

#### `create_peer_connection`
Crée une nouvelle peer connection.

```javascript
await invoke("plugin:crabcamera|create_peer_connection", {
    peerId: "peer-123",
    config: {
        ice_servers: [
            "stun:stun.l.google.com:19302",
            "stun:stun1.l.google.com:19302"
        ]
    }
});
```

#### `create_webrtc_offer`
Génère une offre SDP.

```javascript
const offer = await invoke("plugin:crabcamera|create_webrtc_offer", {
    peerId: "peer-123"
});

// offer = { sdp_type: "offer", sdp: "v=0\r\no=..." }
```

#### `create_webrtc_answer`
Génère une réponse SDP (après avoir reçu une offre distante).

```javascript
const answer = await invoke("plugin:crabcamera|create_webrtc_answer", {
    peerId: "peer-123"
});
```

#### `set_remote_description`
Définit la description SDP distante.

```javascript
await invoke("plugin:crabcamera|set_remote_description", {
    peerId: "peer-123",
    description: {
        sdp_type: "answer",
        sdp: "v=0\r\n..."
    }
});
```

#### `add_ice_candidate`
Ajoute un candidat ICE.

```javascript
await invoke("plugin:crabcamera|add_ice_candidate", {
    peerId: "peer-123",
    candidate: {
        candidate: "candidate:1 1 UDP 2130706431 192.168.1.100 54321 typ host",
        sdp_mid: "0",
        sdp_m_line_index: 0
    }
});
```

#### `get_local_ice_candidates`
Récupère les candidats ICE locaux.

```javascript
const candidates = await invoke("plugin:crabcamera|get_local_ice_candidates", {
    peerId: "peer-123"
});
```

#### `close_peer_connection`
Ferme une peer connection.

```javascript
await invoke("plugin:crabcamera|close_peer_connection", {
    peerId: "peer-123"
});
```

#### `get_peer_connection_status`
Obtient le statut d'une peer connection.

```javascript
const status = await invoke("plugin:crabcamera|get_peer_connection_status", {
    peerId: "peer-123"
});

console.log(`State: ${status.connection_state}`);
```

---

### Transceivers et Data Channels

#### `add_video_transceivers`
Ajoute des transceivers vidéo (pour simulcast).

```javascript
// Configuration simple (une seule qualité)
await invoke("plugin:crabcamera|add_video_transceivers", {
    peerId: "peer-123",
    layers: []
});

// Configuration simulcast (multi-qualités)
await invoke("plugin:crabcamera|add_video_transceivers", {
    peerId: "peer-123",
    layers: [
        { rid: "f", width: 1920, height: 1080, bitrate: 4000000, fps: 30 },
        { rid: "h", width: 1280, height: 720, bitrate: 2000000, fps: 30 },
        { rid: "q", width: 640, height: 360, bitrate: 500000, fps: 15 }
    ]
});
```

#### `create_data_channel`
Crée un canal de données bidirectionnel.

```javascript
await invoke("plugin:crabcamera|create_data_channel", {
    peerId: "peer-123",
    channelLabel: "chat"
});
```

#### `send_data_channel_message`
Envoie un message via un data channel.

```javascript
const encoder = new TextEncoder();
const message = encoder.encode("Hello from browser!");

await invoke("plugin:crabcamera|send_data_channel_message", {
    peerId: "peer-123",
    channelLabel: "chat",
    message: Array.from(message)
});
```

---

## 🎨 Exemple Complet: Application de Vidéoconférence

### HTML

```html
<!DOCTYPE html>
<html>
<head>
    <title>CrabCamera Conference</title>
    <style>
        .video-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: 1rem; }
        video { width: 100%; background: #000; border-radius: 8px; }
        .controls { margin: 1rem 0; }
        button { padding: 0.5rem 1rem; margin: 0.25rem; }
    </style>
</head>
<body>
    <h1>🦀 CrabCamera Conference</h1>
    <div class="controls">
        <button onclick="startCamera()">Start Camera</button>
        <button onclick="stopCamera()">Stop Camera</button>
        <select id="camera-select"></select>
        <select id="quality-select">
            <option value="hd">HD 720p</option>
            <option value="fhd">Full HD 1080p</option>
            <option value="4k">4K 2160p</option>
        </select>
    </div>
    <div class="video-grid">
        <video id="local-video" autoplay muted></video>
    </div>
    <script type="module" src="./conference.js"></script>
</body>
</html>
```

### JavaScript (conference.js)

```javascript
import { invoke } from "@tauri-apps/api/core";

let currentPeer = null;
let currentStream = null;

// Initialiser les caméras disponibles
async function init() {
    await invoke("plugin:crabcamera|initialize_camera_system");
    const cameras = await invoke("plugin:crabcamera|get_available_cameras");

    const select = document.getElementById("camera-select");
    cameras.forEach(cam => {
        const opt = document.createElement("option");
        opt.value = cam.id;
        opt.textContent = cam.name;
        select.appendChild(opt);
    });
}

// Démarrer la caméra
window.startCamera = async function() {
    const deviceId = document.getElementById("camera-select").value;
    const quality = document.getElementById("quality-select").value;

    const configs = {
        hd: { width: 1280, height: 720, bitrate: 2000000 },
        fhd: { width: 1920, height: 1080, bitrate: 4000000 },
        "4k": { width: 3840, height: 2160, bitrate: 8000000 }
    };

    const config = configs[quality];
    const streamId = "stream-" + Date.now();
    const peerId = "peer-" + Date.now();

    try {
        // Démarrer le stream
        await invoke("plugin:crabcamera|start_webrtc_stream", {
            deviceId,
            streamId,
            config: { ...config, fps: 30.0, codec: "h264" },
            mode: "RealCamera"
        });

        // Créer peer connection
        await invoke("plugin:crabcamera|create_peer_connection", {
            peerId,
            config: { ice_servers: ["stun:stun.l.google.com:19302"] }
        });

        // Ajouter transceivers
        await invoke("plugin:crabcamera|add_video_transceivers", {
            peerId,
            layers: []
        });

        // Associer
        await invoke("plugin:crabcamera|associate_stream_with_peer", {
            streamId,
            peerId
        });

        // WebRTC handshake
        const pc = new RTCPeerConnection({
            iceServers: [{ urls: "stun:stun.l.google.com:19302" }]
        });

        const offer = await invoke("plugin:crabcamera|create_webrtc_offer", { peerId });
        await pc.setRemoteDescription({ type: offer.sdp_type, sdp: offer.sdp });

        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);

        await invoke("plugin:crabcamera|set_remote_description", {
            peerId,
            description: { sdp_type: "answer", sdp: answer.sdp }
        });

        pc.onicecandidate = async (e) => {
            if (e.candidate) {
                await invoke("plugin:crabcamera|add_ice_candidate", {
                    peerId,
                    candidate: {
                        candidate: e.candidate.candidate,
                        sdp_mid: e.candidate.sdpMid || "",
                        sdp_m_line_index: e.candidate.sdpMLineIndex || 0
                    }
                });
            }
        };

        pc.ontrack = (e) => {
            document.getElementById("local-video").srcObject = e.streams[0];
        };

        currentPeer = { pc, peerId };
        currentStream = streamId;

        console.log("✅ Camera started successfully");
    } catch (error) {
        console.error("❌ Error:", error);
        alert("Failed to start camera: " + error);
    }
};

// Arrêter la caméra
window.stopCamera = async function() {
    if (currentPeer) {
        currentPeer.pc.close();
        await invoke("plugin:crabcamera|close_peer_connection", {
            peerId: currentPeer.peerId
        });
    }

    if (currentStream) {
        await invoke("plugin:crabcamera|stop_webrtc_stream", {
            streamId: currentStream
        });
    }

    document.getElementById("local-video").srcObject = null;
    currentPeer = null;
    currentStream = null;

    console.log("✅ Camera stopped");
};

// Initialiser au chargement
init();
```

---

## 🔧 Dépannage

### Problème: Pas de vidéo reçue

**Vérifications:**
1. Le stream est-il démarré? `list_webrtc_streams()`
2. La peer connection est-elle établie? `get_peer_connection_status()`
3. Les transceivers sont-ils ajoutés? Vérifier les logs
4. L'association stream/peer est-elle faite? `associate_stream_with_peer`

### Problème: ICE connection failed

**Solutions:**
- Vérifier la configuration STUN/TURN
- Tester avec différents serveurs STUN
- Vérifier les règles de pare-feu
- Consulter les candidats ICE: `get_local_ice_candidates()`

### Problème: Qualité vidéo médiocre

**Ajustements:**
```javascript
// Augmenter le bitrate
await invoke("plugin:crabcamera|set_webrtc_stream_bitrate", {
    streamId: "my-stream",
    bitrate: 5000000  // 5 Mbps
});

// Modifier la configuration
await invoke("plugin:crabcamera|update_webrtc_config", {
    streamId: "my-stream",
    config: {
        width: 1920,
        height: 1080,
        fps: 60.0,
        bitrate: 8000000,
        codec: "h264"
    }
});
```

---

## 📊 Monitoring et Statistiques

```javascript
// Obtenir les stats du stream
const streamStats = await invoke("plugin:crabcamera|get_webrtc_stream_status", {
    streamId: "my-stream"
});

console.log(`
    Résolution: ${streamStats.width}x${streamStats.height}
    FPS: ${streamStats.fps}
    Bitrate: ${streamStats.bitrate / 1000000} Mbps
    Frames encodées: ${streamStats.frames_encoded}
    Octets envoyés: ${(streamStats.bytes_sent / 1024 / 1024).toFixed(2)} MB
    Actif: ${streamStats.is_active}
`);

// Stats de la peer connection
const peerStats = await invoke("plugin:crabcamera|get_peer_connection_status", {
    peerId: "peer-123"
});

console.log(`État de connexion: ${peerStats.connection_state}`);

// Stats système globales
const systemStats = await invoke("plugin:crabcamera|get_webrtc_system_status");
console.log(`
    Streams actifs: ${systemStats.active_streams}
    Peers connectés: ${systemStats.connected_peers}
`);
```

---

## 🎯 Best Practices

1. **Toujours fermer les connexions**
   ```javascript
   window.addEventListener("beforeunload", async () => {
       await stopCamera();
   });
   ```

2. **Gérer les erreurs ICE**
   ```javascript
   pc.oniceconnectionstatechange = () => {
       if (pc.iceConnectionState === "failed") {
           console.error("ICE connection failed, restarting...");
           // Implémenter la reconnexion
       }
   };
   ```

3. **Adapter la qualité au réseau**
   ```javascript
   async function adaptQuality(rtt) {
       const bitrate = rtt < 50 ? 4000000 :
                      rtt < 100 ? 2000000 :
                      1000000;

       await invoke("plugin:crabcamera|set_webrtc_stream_bitrate", {
           streamId,
           bitrate
       });
   }
   ```

4. **Monitorer les performances**
   ```javascript
   setInterval(async () => {
       const stats = await invoke("plugin:crabcamera|get_webrtc_stream_status", {
           streamId
       });
       updateUI(stats);
   }, 1000);
   ```

---

Pour plus d'informations, consultez la [documentation complète](./CRABCAMERA_MANAGER_GUIDE.md).
