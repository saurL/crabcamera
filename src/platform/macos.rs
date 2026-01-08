use crate::errors::CameraError;
use crate::types::{CameraDeviceInfo, CameraFormat, CameraFrame, CameraInitParams};
use nokhwa::{
    pixel_format::RgbFormat,
    query,
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

/// List available cameras on macOS
pub fn list_cameras() -> Result<Vec<CameraDeviceInfo>, CameraError> {
    let cameras = query(nokhwa::utils::ApiBackend::AVFoundation)
        .map_err(|e| CameraError::InitializationError(format!("Failed to query cameras: {}", e)))?;

    let mut device_list = Vec::new();
    for camera_info in cameras {
        let mut device =
            CameraDeviceInfo::new(camera_info.index().to_string(), camera_info.human_name());

        device = device.with_description(camera_info.description().to_string());

        // Add common macOS camera formats
        let formats = vec![
            CameraFormat::new(1920, 1080, 30.0),
            CameraFormat::new(1280, 720, 30.0),
            CameraFormat::new(640, 480, 30.0),
        ];
        device = device.with_formats(formats);

        device_list.push(device);
    }

    Ok(device_list)
}

/// Initialize camera on macOS with AVFoundation backend
///
/// Uses nokhwa's CameraFormat API (0.10.x) with MJPEG frame format
/// for broad compatibility across macOS camera hardware.
pub fn initialize_camera(params: CameraInitParams) -> Result<MacOSCamera, CameraError> {
    let device_index = params
        .device_id
        .parse::<u32>()
        .map_err(|_| CameraError::InitializationError("Invalid device ID".to_string()))?;

    // Create requested format using nokhwa 0.10.x CameraFormat API
    // Note: CameraFormat::new takes (Resolution, FrameFormat, fps)
    // Using MJPEG for broad hardware compatibility on macOS
    let requested_format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(
        nokhwa::utils::CameraFormat::new(
            nokhwa::utils::Resolution::new(params.format.width, params.format.height),
            nokhwa::utils::FrameFormat::MJPEG,
            params.format.fps as u32,
        ),
    ));
    let camera = Camera::new(
        nokhwa::utils::CameraIndex::Index(device_index),
        requested_format,
    )
    .map_err(|e| CameraError::InitializationError(format!("Failed to initialize camera: {}", e)))?;

    Ok(MacOSCamera {
        camera: Arc::new(Mutex::new(camera)),
        device_id: params.device_id,
        format: params.format,
        is_streaming: Arc::new(AtomicBool::new(false)),
        frame_callback: Arc::new(RwLock::new(None)),
    })
}

/// macOS-specific camera wrapper
pub struct MacOSCamera {
    camera: Arc<Mutex<Camera>>,
    device_id: String,
    format: CameraFormat,
    is_streaming: Arc<AtomicBool>,
    frame_callback: Arc<RwLock<Option<Arc<dyn Fn(CameraFrame) + Send + Sync>>>>,
}

impl MacOSCamera {
    /// Capture frame from macOS camera using AVFoundation
    pub fn capture_frame(&self) -> Result<CameraFrame, CameraError> {
        let mut camera = self
            .camera
            .lock()
            .map_err(|_| CameraError::CaptureError("Failed to lock camera".to_string()))?;

        let frame = camera
            .frame()
            .map_err(|e| CameraError::CaptureError(format!("Failed to capture frame: {}", e)))?;

        let camera_frame = CameraFrame::new(
            frame.buffer_bytes().to_vec(),
            frame.resolution().width_x,
            frame.resolution().height_y,
            self.device_id.clone(),
        );

        Ok(camera_frame.with_format("RGB8".to_string()))
    }

    /// Get current format
    pub fn get_format(&self) -> &CameraFormat {
        &self.format
    }

    /// Get device ID
    pub fn get_device_id(&self) -> &str {
        &self.device_id
    }

    /// Check if camera is available
    pub fn is_available(&self) -> bool {
        self.camera
            .lock()
            .map(|c| c.is_stream_open())
            .unwrap_or(false)
    }

    /// Set frame callback for continuous capture
    pub fn set_frame_callback<F>(&self, callback: F)
    where
        F: Fn(CameraFrame) + Send + Sync + 'static,
    {
        let mut cb = self.frame_callback.write().unwrap();
        *cb = Some(Arc::new(callback));
    }

    /// Clear frame callback
    pub fn clear_frame_callback(&self) {
        let mut cb = self.frame_callback.write().unwrap();
        *cb = None;
    }

    /// Start camera stream
    pub fn start_stream(&mut self) -> Result<(), CameraError> {
        let mut camera = self
            .camera
            .lock()
            .map_err(|_| CameraError::InitializationError("Failed to lock camera".to_string()))?;

        camera.open_stream().map_err(|e| {
            CameraError::InitializationError(format!("Failed to start stream: {}", e))
        })?;

        // Start callback thread if callback is set

        Ok(())
    }

    /// Start streaming camera frames
    pub async fn start_streaming(
        &self,
        callback: Box<dyn Fn(CameraFrame) + Send + Sync>,
    ) -> Result<(), String> {
        let mut camera = self
            .camera
            .lock()
            .map_err(|_| CameraError::InitializationError("Failed to lock camera".to_string()))?;

        camera.open_stream().map_err(|e| {
            CameraError::InitializationError(format!("Failed to start stream: {}", e))
        })?;

        // Set streaming flag
        self.is_streaming.store(true, Ordering::SeqCst);

        // Spawn a thread to continuously capture frames and call the callback
        let camera_clone = self.camera.clone();
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

                        // Call the callback if still set
                        callback(camera_frame);
                    }
                    Err(e) => {
                        eprintln!("Error capturing frame in callback: {}", e);
                        // Don't break immediately, wait and retry
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                // Limit to ~30 FPS
                std::thread::sleep(std::time::Duration::from_millis(33));
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
        // Stop callback thread
        if self.is_streaming.load(Ordering::SeqCst) == true {
            self.is_streaming.store(false, Ordering::SeqCst);

            // Wait for thread to stop
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let mut camera = self
            .camera
            .lock()
            .map_err(|_| CameraError::InitializationError("Failed to lock camera".to_string()))?;

        camera.stop_stream().map_err(|e| {
            CameraError::InitializationError(format!("Failed to stop stream: {}", e))
        })?;

        Ok(())
    }

    /// Get camera controls (stub for macOS - not yet implemented)
    pub fn get_controls(&self) -> Result<crate::types::CameraControls, CameraError> {
        // macOS AVFoundation controls would be queried here
        // For now, return default controls
        Ok(crate::types::CameraControls::default())
    }

    /// Apply camera controls (stub for macOS - not yet implemented)
    pub fn apply_controls(
        &mut self,
        _controls: &crate::types::CameraControls,
    ) -> Result<(), CameraError> {
        // macOS AVFoundation control application would happen here
        Ok(())
    }

    /// Test camera capabilities (stub for macOS)
    pub fn test_capabilities(&self) -> Result<crate::types::CameraCapabilities, CameraError> {
        Ok(crate::types::CameraCapabilities::default())
    }

    /// Get performance metrics (stub for macOS)
    pub fn get_performance_metrics(
        &self,
    ) -> Result<crate::types::CameraPerformanceMetrics, CameraError> {
        Ok(crate::types::CameraPerformanceMetrics::default())
    }
}

// Ensure the camera is properly cleaned up
impl Drop for MacOSCamera {
    fn drop(&mut self) {
        if let Ok(mut camera) = self.camera.lock() {
            let _ = camera.stop_stream();
        }
    }
}

// Thread-safe implementation
unsafe impl Send for MacOSCamera {}
unsafe impl Sync for MacOSCamera {}
