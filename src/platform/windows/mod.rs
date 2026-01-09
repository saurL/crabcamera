// Windows platform implementation combining nokhwa capture with MediaFoundation controls

pub mod capture;
pub mod controls;

use self::controls::MediaFoundationControls;
use crate::errors::CameraError;
use crate::types::{CameraCapabilities, CameraControls, CameraFormat, CameraFrame};
use nokhwa::Camera;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

/// Combined Windows camera interface with both capture and control capabilities
pub struct WindowsCamera {
    /// nokhwa camera for frame capture
    pub nokhwa_camera: Arc<Mutex<Camera>>,
    /// MediaFoundation controls for advanced camera settings
    pub mf_controls: MediaFoundationControls,
    /// Device identifier
    pub device_id: String,
    /// Streaming flag for callback thread
    is_streaming: Arc<AtomicBool>,
}

impl WindowsCamera {
    /// Create new Windows camera with both capture and control capabilities
    pub fn new(device_id: String, format: CameraFormat) -> Result<Self, CameraError> {
        log::info!(
            "Initializing Windows camera {} with MediaFoundation controls",
            device_id
        );

        // Initialize nokhwa camera for capture
        let nokhwa_camera = capture::initialize_camera(&device_id, format)?;

        // Initialize MediaFoundation controls
        let device_index = device_id
            .parse::<u32>()
            .map_err(|_| CameraError::InitializationError("Invalid device ID".to_string()))?;
        let mf_controls = MediaFoundationControls::new(device_index)?;

        Ok(WindowsCamera {
            nokhwa_camera: Arc::new(Mutex::new(nokhwa_camera)),
            mf_controls,
            device_id,
            is_streaming: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Capture a frame using nokhwa
    pub fn capture_frame(&mut self) -> Result<CameraFrame, CameraError> {
        let mut camera = self
            .nokhwa_camera
            .lock()
            .map_err(|_| CameraError::CaptureError("Failed to lock camera".to_string()))?;
        capture::capture_frame(&mut *camera, &self.device_id)
    }

    /// Apply camera controls using MediaFoundation
    pub fn apply_controls(
        &mut self,
        controls: &CameraControls,
    ) -> Result<Vec<String>, CameraError> {
        self.mf_controls.apply_controls(controls)
    }

    /// Get current camera control values
    pub fn get_controls(&self) -> Result<CameraControls, CameraError> {
        self.mf_controls.get_controls()
    }

    /// Test camera capabilities
    pub fn test_capabilities(&self) -> Result<CameraCapabilities, CameraError> {
        self.mf_controls.get_capabilities()
    }

    /// Start camera stream
    pub fn start_stream(&mut self) -> Result<(), CameraError> {
        log::debug!("Opening camera stream for device {}", self.device_id);

        let mut camera = self
            .nokhwa_camera
            .lock()
            .map_err(|_| CameraError::StreamError("Failed to lock camera".to_string()))?;

        camera
            .open_stream()
            .map_err(|e| CameraError::StreamError(format!("Failed to open stream: {}", e)))?;

        Ok(())
    }

    /// Start streaming camera frames
    pub async fn start_streaming(
        &self,
        callback: Box<dyn Fn(CameraFrame) + Send + Sync>,
    ) -> Result<(), CameraError> {
        log::debug!("Opening camera stream for device {}", self.device_id);

        let mut camera = self
            .nokhwa_camera
            .lock()
            .map_err(|_| CameraError::StreamError("Failed to lock camera".to_string()))?;

        camera
            .open_stream()
            .map_err(|e| CameraError::StreamError(format!("Failed to open stream: {}", e)))?;

        // Set streaming flag
        self.is_streaming.store(true, Ordering::SeqCst);

        // Spawn a thread to continuously capture frames and call the callback
        let camera_clone = self.nokhwa_camera.clone();
        let device_id = self.device_id.clone();
        let is_streaming = self.is_streaming.clone();
        let callback = Arc::new(callback);

        std::thread::spawn(move || {
            while is_streaming.load(Ordering::SeqCst) {
                // Temporary lock to capture frame
                let frame_result = {
                    let mut cam = match camera_clone.lock() {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Failed to lock camera: {}", e);
                            break;
                        }
                    };

                    cam.frame()
                }; // Lock released here

                match frame_result {
                    Ok(frame) => {
                        let camera_frame = CameraFrame::new(
                            frame.buffer_bytes().to_vec(),
                            frame.resolution().width_x,
                            frame.resolution().height_y,
                            device_id.clone(),
                        )
                        .with_format("RGB8".to_string());

                        callback(camera_frame);
                    }
                    Err(e) => {
                        eprintln!("Error capturing frame in callback: {}", e);
                        // Don't break immediately, wait and retry
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                // Limit to ~60 FPS
                std::thread::sleep(std::time::Duration::from_millis(16));
            }

            log::info!(
                "Callback streaming thread stopped for device: {}",
                device_id
            );
        });

        Ok(())
    }

    /// Stop camera stream
    pub fn stop_stream(&mut self) -> Result<(), CameraError> {
        log::debug!("Stopping camera stream for device {}", self.device_id);
        if self.is_streaming.load(Ordering::SeqCst) == true {
            // Stop callback thread
            self.is_streaming.store(false, Ordering::SeqCst);

            // Wait for thread to stop
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut camera = self
            .nokhwa_camera
            .lock()
            .map_err(|_| CameraError::StreamError("Failed to lock camera".to_string()))?;

        camera
            .stop_stream()
            .map_err(|e| CameraError::StreamError(format!("Failed to stop stream: {}", e)))
    }

    /// Check if the stream is currently open
    pub fn is_stream_open(&self) -> bool {
        self.nokhwa_camera
            .lock()
            .map(|c| c.is_stream_open())
            .unwrap_or(false)
    }

    /// Check if camera is available
    pub fn is_available(&self) -> bool {
        // Camera availability is determined by successful initialization
        // Since the Camera object was created successfully, it's available
        // For more robust checking, we could attempt a test frame capture
        true
    }

    /// Get device ID
    pub fn get_device_id(&self) -> &str {
        &self.device_id
    }
}

// Re-export public interface functions for compatibility
pub use capture::{capture_frame, initialize_camera, list_cameras};
