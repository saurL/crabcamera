use crate::platform::{optimizations, PlatformCamera};
use crate::types::{CameraFormat, CameraFrame, CameraInitParams};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, AppHandle, Emitter, Runtime};
use tokio::sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock};

// Registry of active streaming sessions
lazy_static::lazy_static! {
    static ref STREAMING_SESSIONS: Arc<AsyncRwLock<HashMap<String, StreamingSession>>> =
        Arc::new(AsyncRwLock::new(HashMap::new()));
}

/// Streaming session state
struct StreamingSession {
    camera: Arc<AsyncMutex<PlatformCamera>>,
    is_active: Arc<AsyncRwLock<bool>>,
}

/// Configuration for starting a stream
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamConfig {
    pub device_id: String,
    /// Use optimal settings (recommended)
    #[serde(default = "default_true")]
    pub use_optimal_settings: bool,
    /// Custom format (if use_optimal_settings = false)
    pub format: Option<CameraFormat>,
}

fn default_true() -> bool {
    true
}

/// Status of a streaming session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamStatus {
    pub device_id: String,
    pub is_active: bool,
}

/// Start direct streaming (frames via Tauri events)
#[command]
pub async fn start_direct_streaming<R: Runtime>(
    app: AppHandle<R>,
    config: StreamConfig,
) -> Result<String, String> {
    log::info!("Starting direct streaming for device: {}", config.device_id);

    let mut sessions = STREAMING_SESSIONS.write().await;

    // Check if already streaming
    if sessions.contains_key(&config.device_id) {
        return Err(format!(
            "Streaming already active for device {}",
            config.device_id
        ));
    }

    // Initialize camera with optimal or custom settings
    let params = if config.use_optimal_settings {
        log::info!("Using optimal settings for device {}", config.device_id);
        let mut optimal = optimizations::get_optimal_settings();
        optimal.device_id = config.device_id.clone();
        optimal
    } else {
        let format = config
            .format
            .unwrap_or_else(|| optimizations::get_photography_format());
        CameraInitParams::new(config.device_id.clone()).with_format(format)
    };

    let camera = PlatformCamera::new(params)
        .map_err(|e| format!("Failed to initialize camera: {}", e))?;

    let camera_arc = Arc::new(AsyncMutex::new(camera));
    let is_active = Arc::new(AsyncRwLock::new(true));

    // Create callback closure that emits frames to Tauri frontend
    let app_clone = app.clone();
    let callback = Box::new(move |frame: CameraFrame| {
        // Emit frame to frontend via Tauri event
        if let Err(e) = app_clone.emit("camera-frame", &frame) {
            log::error!("Failed to emit frame event: {}", e);
        }
    });

    // Start streaming with the callback
    {
        let cam = camera_arc.lock().await;
        cam.start_streaming(callback)
            .await
            .map_err(|e| format!("Failed to start streaming: {}", e))?;
    }

    // Store session
    sessions.insert(
        config.device_id.clone(),
        StreamingSession {
            camera: camera_arc,
            is_active,
        },
    );

    Ok(format!(
        "Direct streaming started for device {}",
        config.device_id
    ))
}

/// Stop direct streaming
#[command]
pub async fn stop_direct_streaming(device_id: String) -> Result<String, String> {
    log::info!("Stopping direct streaming for device: {}", device_id);

    let mut sessions = STREAMING_SESSIONS.write().await;

    if let Some(session) = sessions.remove(&device_id) {
        // Mark as inactive
        *session.is_active.write().await = false;

        // Stop the stream
        let mut cam = session.camera.lock().await;
        cam.stop_stream()
            .map_err(|e| format!("Failed to stop stream: {}", e))?;

        Ok(format!("Direct streaming stopped for device {}", device_id))
    } else {
        Err(format!("No active streaming session for device {}", device_id))
    }
}

/// Get streaming status
#[command]
pub async fn get_streaming_status(device_id: String) -> Result<StreamStatus, String> {
    let sessions = STREAMING_SESSIONS.read().await;

    if let Some(session) = sessions.get(&device_id) {
        let is_active = *session.is_active.read().await;
        Ok(StreamStatus {
            device_id,
            is_active,
        })
    } else {
        Ok(StreamStatus {
            device_id,
            is_active: false,
        })
    }
}

/// List all active streaming sessions
#[command]
pub async fn list_active_streams() -> Result<Vec<StreamStatus>, String> {
    let sessions = STREAMING_SESSIONS.read().await;
    let mut statuses = Vec::new();

    for (device_id, session) in sessions.iter() {
        let is_active = *session.is_active.read().await;
        statuses.push(StreamStatus {
            device_id: device_id.clone(),
            is_active,
        });
    }

    Ok(statuses)
}

/// Release all streaming sessions (cleanup)
#[command]
pub async fn release_all_streams() -> Result<String, String> {
    log::info!("Releasing all streaming sessions");

    let mut sessions = STREAMING_SESSIONS.write().await;
    let count = sessions.len();

    for (device_id, session) in sessions.drain() {
        // Mark as inactive
        *session.is_active.write().await = false;

        // Stop stream
        let mut cam = session.camera.lock().await;
        if let Err(e) = cam.stop_stream() {
            log::warn!("Error stopping stream for {}: {}", device_id, e);
        }
    }

    Ok(format!("Released {} streaming session(s)", count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_config_defaults() {
        let config = StreamConfig {
            device_id: "0".to_string(),
            use_optimal_settings: true,
            format: None,
        };

        assert_eq!(config.device_id, "0");
        assert!(config.use_optimal_settings);
        assert!(config.format.is_none());
    }

    #[tokio::test]
    async fn test_stream_status() {
        let status = StreamStatus {
            device_id: "0".to_string(),
            is_active: true,
        };

        assert_eq!(status.device_id, "0");
        assert!(status.is_active);
    }
}
