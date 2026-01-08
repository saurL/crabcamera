#[cfg(feature = "audio")]
use crate::commands::audio;
#[cfg(feature = "recording")]
use crate::commands::recording;
#[cfg(feature = "webrtc")]
use crate::commands::webrtc;
use crate::commands::{
    advanced, capture, config, device_monitor, focus_stack, init, permissions, quality, streaming,
};

use crate::desktop::webrtc::WebRTCSystemStatus;
use crate::types::*;
use crate::webrtc::streaming::StreamStats;
use crate::webrtc::RTCConfiguration;
use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<CrabCameraManager<R>> {
    Ok(CrabCameraManager(app.clone()))
}

/// CrabCamera Manager - Rust API for all camera operations
///
/// This struct provides direct Rust access to all CrabCamera functionality
/// without needing to go through Tauri commands. All methods mirror the
/// command functions in `src/commands/`.
pub struct CrabCameraManager<R: Runtime>(AppHandle<R>);

impl<R: Runtime> CrabCameraManager<R> {
    // ==================== Initialization Commands ====================

    /// Initialize the camera system
    pub async fn initialize_camera_system(&self) -> Result<String, String> {
        init::initialize_camera_system().await
    }

    /// Get list of available cameras
    pub async fn get_available_cameras(&self) -> Result<Vec<CameraDeviceInfo>, String> {
        init::get_available_cameras().await
    }

    /// Get platform information
    pub async fn get_platform_info(&self) -> Result<crate::platform::PlatformInfo, String> {
        init::get_platform_info().await
    }

    /// Test camera system
    pub async fn test_camera_system(&self) -> Result<crate::platform::SystemTestResult, String> {
        init::test_camera_system().await
    }

    /// Get current platform
    pub async fn get_current_platform(&self) -> Result<String, String> {
        init::get_current_platform().await
    }

    /// Check if a specific camera is available
    pub async fn check_camera_availability(&self, device_id: String) -> Result<bool, String> {
        init::check_camera_availability(device_id).await
    }

    /// Get supported formats for a camera
    pub async fn get_camera_formats(&self, device_id: String) -> Result<Vec<CameraFormat>, String> {
        init::get_camera_formats(device_id).await
    }

    /// Get recommended format for current platform
    pub async fn get_recommended_format(&self) -> Result<CameraFormat, String> {
        init::get_recommended_format().await
    }

    /// Get optimal camera settings
    pub async fn get_optimal_settings(&self) -> Result<CameraInitParams, String> {
        init::get_optimal_settings().await
    }

    /// Get system diagnostics
    pub async fn get_system_diagnostics(&self) -> Result<init::SystemDiagnostics, String> {
        init::get_system_diagnostics().await
    }

    // ==================== Capture Commands ====================

    /// Capture a single photo
    pub async fn capture_single_photo(
        &self,
        device_id: Option<String>,
        format: Option<CameraFormat>,
    ) -> Result<CameraFrame, String> {
        capture::capture_single_photo(device_id, format).await
    }

    /// Capture a sequence of photos
    pub async fn capture_photo_sequence(
        &self,
        device_id: String,
        count: u32,
        interval_ms: u32,
        format: Option<CameraFormat>,
    ) -> Result<Vec<CameraFrame>, String> {
        capture::capture_photo_sequence(device_id, count, interval_ms, format).await
    }

    /// Capture with quality retry
    pub async fn capture_with_quality_retry(
        &self,
        device_id: Option<String>,
        max_attempts: Option<u32>,
        min_quality_score: Option<f32>,
        format: Option<CameraFormat>,
    ) -> Result<CameraFrame, String> {
        capture::capture_with_quality_retry(device_id, max_attempts, min_quality_score, format)
            .await
    }

    /// Start camera preview
    pub async fn start_camera_preview(
        &self,
        device_id: String,
        format: Option<CameraFormat>,
    ) -> Result<String, String> {
        capture::start_camera_preview(device_id, format).await
    }

    /// Stop camera preview
    pub async fn stop_camera_preview(&self, device_id: String) -> Result<String, String> {
        capture::stop_camera_preview(device_id).await
    }

    /// Release camera resources
    pub async fn release_camera(&self, device_id: String) -> Result<String, String> {
        capture::release_camera(device_id).await
    }

    /// Get capture statistics
    pub async fn get_capture_stats(
        &self,
        device_id: String,
    ) -> Result<capture::CaptureStats, String> {
        capture::get_capture_stats(device_id).await
    }

    /// Save frame to disk
    pub async fn save_frame_to_disk(
        &self,
        frame: CameraFrame,
        file_path: String,
    ) -> Result<String, String> {
        capture::save_frame_to_disk(frame, file_path).await
    }

    /// Save frame compressed
    pub async fn save_frame_compressed(
        &self,
        frame: CameraFrame,
        file_path: String,
        quality: Option<u8>,
    ) -> Result<String, String> {
        capture::save_frame_compressed(frame, file_path, quality).await
    }

    // ==================== Advanced Camera Controls ====================

    /// Set camera controls
    pub async fn set_camera_controls(
        &self,
        device_id: String,
        controls: CameraControls,
    ) -> Result<String, String> {
        advanced::set_camera_controls(device_id, controls).await
    }

    /// Get camera controls
    pub async fn get_camera_controls(&self, device_id: String) -> Result<CameraControls, String> {
        advanced::get_camera_controls(device_id).await
    }

    /// Capture burst sequence
    pub async fn capture_burst_sequence(
        &self,
        device_id: String,
        config: BurstConfig,
    ) -> Result<Vec<CameraFrame>, String> {
        advanced::capture_burst_sequence(device_id, config).await
    }

    /// Set manual focus
    pub async fn set_manual_focus(
        &self,
        device_id: String,
        focus_distance: f32,
    ) -> Result<String, String> {
        advanced::set_manual_focus(device_id, focus_distance).await
    }

    /// Set manual exposure
    pub async fn set_manual_exposure(
        &self,
        device_id: String,
        exposure_time: f32,
        iso_sensitivity: u32,
    ) -> Result<String, String> {
        advanced::set_manual_exposure(device_id, exposure_time, iso_sensitivity).await
    }

    /// Set white balance
    pub async fn set_white_balance(
        &self,
        device_id: String,
        white_balance: WhiteBalance,
    ) -> Result<String, String> {
        advanced::set_white_balance(device_id, white_balance).await
    }

    /// Capture HDR sequence
    pub async fn capture_hdr_sequence(
        &self,
        device_id: String,
    ) -> Result<Vec<CameraFrame>, String> {
        advanced::capture_hdr_sequence(device_id).await
    }

    /// Capture focus stack (legacy)
    pub async fn capture_focus_stack_legacy(
        &self,
        device_id: String,
        num_steps: u32,
    ) -> Result<Vec<CameraFrame>, String> {
        advanced::capture_focus_stack_legacy(device_id, num_steps).await
    }

    /// Get camera performance metrics
    pub async fn get_camera_performance(
        &self,
        device_id: String,
    ) -> Result<CameraPerformanceMetrics, String> {
        advanced::get_camera_performance(device_id).await
    }

    /// Test camera capabilities
    pub async fn test_camera_capabilities(
        &self,
        device_id: String,
    ) -> Result<CameraCapabilities, String> {
        advanced::test_camera_capabilities(device_id).await
    }

    // ==================== Quality Validation Commands ====================

    /// Validate frame quality
    pub async fn validate_frame_quality(
        &self,
        device_id: Option<String>,
        capture_format: Option<crate::types::CameraFormat>,
    ) -> Result<crate::quality::QualityReport, String> {
        quality::validate_frame_quality(device_id, capture_format).await
    }

    /// Validate provided frame
    pub async fn validate_provided_frame(
        &self,
        frame: CameraFrame,
    ) -> Result<crate::quality::QualityReport, String> {
        quality::validate_provided_frame(frame).await
    }

    /// Analyze frame blur
    pub async fn analyze_frame_blur(
        &self,
        device_id: Option<String>,
        capture_format: Option<crate::types::CameraFormat>,
    ) -> Result<crate::quality::BlurMetrics, String> {
        quality::analyze_frame_blur(device_id, capture_format).await
    }

    /// Analyze frame exposure
    pub async fn analyze_frame_exposure(
        &self,
        device_id: Option<String>,
        capture_format: Option<crate::types::CameraFormat>,
    ) -> Result<crate::quality::ExposureMetrics, String> {
        quality::analyze_frame_exposure(device_id, capture_format).await
    }

    /// Update quality configuration
    pub async fn update_quality_config(
        &self,
        config: quality::ValidationConfigDto,
    ) -> Result<String, String> {
        quality::update_quality_config(config).await
    }

    /// Get quality configuration
    pub async fn get_quality_config(&self) -> Result<quality::ValidationConfigDto, String> {
        quality::get_quality_config().await
    }

    /// Capture best quality frame
    pub async fn capture_best_quality_frame(
        &self,
        device_id: Option<String>,
        capture_format: Option<crate::types::CameraFormat>,
        num_attempts: Option<u32>,
    ) -> Result<crate::desktop::quality::CaptureQualityResult, String> {
        quality::capture_best_quality_frame(device_id, capture_format, num_attempts).await
    }

    /// Auto capture with quality validation
    pub async fn auto_capture_with_quality(
        &self,
        device_id: Option<String>,
        capture_format: Option<crate::types::CameraFormat>,
        min_quality_threshold: Option<f32>,
        max_attempts: Option<u32>,
        timeout_seconds: Option<u32>,
    ) -> Result<crate::desktop::quality::CaptureQualityResult, String> {
        quality::auto_capture_with_quality(
            device_id,
            capture_format,
            min_quality_threshold,
            max_attempts,
            timeout_seconds,
        )
        .await
    }

    /// Analyze quality trends
    pub async fn analyze_quality_trends(
        &self,
        device_id: Option<String>,
        capture_format: Option<crate::types::CameraFormat>,
        num_samples: Option<u32>,
    ) -> Result<crate::desktop::quality::QualityTrendAnalysis, String> {
        quality::analyze_quality_trends(device_id, capture_format, num_samples).await
    }

    // ==================== Focus Stack Commands ====================

    /// Capture focus stack
    pub async fn capture_focus_stack(
        &self,
        device_id: String,
        config: crate::focus_stack::FocusStackConfig,
        format: Option<CameraFormat>,
    ) -> Result<crate::focus_stack::FocusStackResult, String> {
        focus_stack::capture_focus_stack(device_id, config, format).await
    }

    /// Capture focus brackets
    pub async fn capture_focus_brackets_command(
        &self,
        device_id: String,
        brackets: u32,
        shots_per_bracket: u32,
        sharpness_threshold: f32,
        blend_levels: u32,
        format: Option<CameraFormat>,
    ) -> Result<crate::focus_stack::FocusStackResult, String> {
        focus_stack::capture_focus_brackets_command(
            device_id,
            brackets,
            shots_per_bracket,
            sharpness_threshold,
            blend_levels,
            format,
        )
        .await
    }

    /// Get default focus configuration
    pub fn get_default_focus_config(&self) -> crate::focus_stack::FocusStackConfig {
        focus_stack::get_default_focus_config()
    }

    /// Validate focus configuration
    pub fn validate_focus_config(
        &self,
        config: crate::focus_stack::FocusStackConfig,
    ) -> Result<String, String> {
        focus_stack::validate_focus_config(config)
    }

    // ==================== Configuration Commands ====================

    /// Get global configuration
    pub async fn get_config(&self) -> Result<crate::config::CrabCameraConfig, String> {
        config::get_config().await
    }

    /// Update global configuration
    pub async fn update_config(
        &self,
        new_config: crate::config::CrabCameraConfig,
    ) -> Result<(), String> {
        config::update_config(new_config).await
    }

    /// Reset configuration to defaults
    pub async fn reset_config(&self) -> Result<crate::config::CrabCameraConfig, String> {
        config::reset_config().await
    }

    /// Get camera configuration
    pub async fn get_camera_config(&self) -> Result<crate::config::CameraConfig, String> {
        config::get_camera_config().await
    }

    /// Get quality configuration (full)
    pub async fn get_full_quality_config(&self) -> Result<crate::config::QualityConfig, String> {
        config::get_full_quality_config().await
    }

    /// Get storage configuration
    pub async fn get_storage_config(&self) -> Result<crate::config::StorageConfig, String> {
        config::get_storage_config().await
    }

    /// Get advanced configuration
    pub async fn get_advanced_config(&self) -> Result<crate::config::AdvancedConfig, String> {
        config::get_advanced_config().await
    }

    /// Update camera configuration
    pub async fn update_camera_config(
        &self,
        camera_config: crate::config::CameraConfig,
    ) -> Result<(), String> {
        config::update_camera_config(camera_config).await
    }

    /// Update quality configuration (full)
    pub async fn update_full_quality_config(
        &self,
        quality_config: crate::config::QualityConfig,
    ) -> Result<(), String> {
        config::update_full_quality_config(quality_config).await
    }

    /// Update storage configuration
    pub async fn update_storage_config(
        &self,
        storage_config: crate::config::StorageConfig,
    ) -> Result<(), String> {
        config::update_storage_config(storage_config).await
    }

    /// Update advanced configuration
    pub async fn update_advanced_config(
        &self,
        advanced_config: crate::config::AdvancedConfig,
    ) -> Result<(), String> {
        config::update_advanced_config(advanced_config).await
    }

    // ==================== Device Monitoring Commands ====================

    /// Start device monitoring
    pub async fn start_device_monitoring(&self) -> Result<String, String> {
        device_monitor::start_device_monitoring().await
    }

    /// Stop device monitoring
    pub async fn stop_device_monitoring(&self) -> Result<String, String> {
        device_monitor::stop_device_monitoring().await
    }

    /// Poll for device events
    pub async fn poll_device_event(
        &self,
    ) -> Result<Option<device_monitor::DeviceEventInfo>, String> {
        device_monitor::poll_device_event().await
    }

    /// Get monitored devices
    pub async fn get_monitored_devices(&self) -> Result<Vec<CameraDeviceInfo>, String> {
        device_monitor::get_monitored_devices().await
    }

    // ==================== Permission Commands ====================

    /// Request camera permission
    pub async fn request_camera_permission(
        &self,
    ) -> Result<crate::permissions::PermissionInfo, String> {
        permissions::request_camera_permission().await
    }

    /// Check camera permission status
    pub async fn check_camera_permission_status(
        &self,
    ) -> Result<crate::permissions::PermissionInfo, String> {
        permissions::check_camera_permission_status().await
    }

    /// Get permission status string
    pub fn get_permission_status_string(&self) -> String {
        permissions::get_permission_status_string()
    }

    // ==================== Streaming Commands ====================

    /// Start direct streaming (frames via Tauri events)
    pub async fn start_direct_streaming(
        &self,
        config: streaming::StreamConfig,
    ) -> Result<String, String> {
        streaming::start_direct_streaming(self.0.clone(), config).await
    }

    /// Stop direct streaming
    pub async fn stop_direct_streaming(&self, device_id: String) -> Result<String, String> {
        streaming::stop_direct_streaming(device_id).await
    }

    /// Get streaming status
    pub async fn get_streaming_status(
        &self,
        device_id: String,
    ) -> Result<streaming::StreamStatus, String> {
        streaming::get_streaming_status(device_id).await
    }

    /// List active streams
    pub async fn list_active_streams(&self) -> Result<Vec<streaming::StreamStatus>, String> {
        streaming::list_active_streams().await
    }

    /// Release all streams
    pub async fn release_all_streams(&self) -> Result<String, String> {
        streaming::release_all_streams().await
    }

    // ==================== Audio Commands (Optional Feature) ====================

    #[cfg(feature = "audio")]
    /// List audio devices
    pub async fn list_audio_devices(
        &self,
    ) -> Result<Vec<crate::commands::audio::AudioDeviceInfo>, String> {
        audio::list_audio_devices().await
    }

    #[cfg(feature = "audio")]
    /// Get default audio device
    pub async fn get_default_audio_device(
        &self,
    ) -> Result<crate::commands::audio::AudioDeviceInfo, String> {
        audio::get_default_audio_device().await
    }

    // ==================== Recording Commands (Optional Feature) ====================

    #[cfg(feature = "recording")]
    /// Start recording session
    pub async fn start_recording(
        &self,
        device_id: Option<String>,
        output_path: String,
        width: u32,
        height: u32,
        fps: f64,
        quality: Option<String>,
        title: Option<String>,
        #[cfg(feature = "audio")] audio_device_id: Option<String>,
    ) -> Result<String, String> {
        recording::start_recording(
            device_id,
            output_path,
            width,
            height,
            fps,
            quality,
            title,
            #[cfg(feature = "audio")]
            audio_device_id,
        )
        .await
    }

    #[cfg(feature = "recording")]
    /// Record a frame
    pub async fn record_frame(&self, session_id: String) -> Result<u64, String> {
        recording::record_frame(session_id).await
    }

    #[cfg(feature = "recording")]
    /// Stop recording
    pub async fn stop_recording(
        &self,
        session_id: String,
    ) -> Result<crate::recording::RecordingStats, String> {
        recording::stop_recording(session_id).await
    }

    #[cfg(feature = "recording")]
    /// Get recording status
    pub async fn get_recording_status(
        &self,
        session_id: String,
    ) -> Result<recording::RecordingStatus, String> {
        recording::get_recording_status(session_id).await
    }

    #[cfg(feature = "recording")]
    /// List recording sessions
    pub async fn list_recording_sessions(&self) -> Result<Vec<String>, String> {
        recording::list_recording_sessions().await
    }

    // ==================== WebRTC Commands (Optional Feature) ====================

    #[cfg(feature = "webrtc")]
    /// Start WebRTC stream
    pub async fn start_webrtc_stream(
        &self,
        device_id: String,
        stream_id: String,
        config: Option<crate::webrtc::streaming::StreamConfig>,
        mode: Option<crate::webrtc::StreamMode>,
    ) -> Result<String, String> {
        webrtc::start_webrtc_stream(device_id, stream_id, config, mode).await
    }

    #[cfg(feature = "webrtc")]
    /// Stop WebRTC stream
    pub async fn stop_webrtc_stream(&self, stream_id: String) -> Result<String, String> {
        webrtc::stop_webrtc_stream(stream_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Get WebRTC stream status
    pub async fn get_webrtc_stream_status(&self, stream_id: String) -> Result<StreamStats, String> {
        webrtc::get_webrtc_stream_status(stream_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Update WebRTC configuration
    pub async fn update_webrtc_config(
        &self,
        stream_id: String,
        config: crate::webrtc::StreamConfig,
    ) -> Result<String, String> {
        webrtc::update_webrtc_config(stream_id, config).await
    }

    #[cfg(feature = "webrtc")]
    /// List WebRTC streams
    pub async fn list_webrtc_streams(&self) -> Result<Vec<StreamStats>, String> {
        webrtc::list_webrtc_streams().await
    }

    #[cfg(feature = "webrtc")]
    /// Associate stream with peer
    pub async fn associate_stream_with_peer(
        &self,
        stream_id: String,
        peer_id: String,
    ) -> Result<String, String> {
        webrtc::associate_stream_with_peer(stream_id, peer_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Create peer connection
    pub async fn create_peer_connection(
        &self,
        peer_id: String,
        config: Option<RTCConfiguration>,
    ) -> Result<String, String> {
        webrtc::create_peer_connection(peer_id, config).await
    }

    #[cfg(feature = "webrtc")]
    /// Create WebRTC offer
    pub async fn create_webrtc_offer(
        &self,
        peer_id: String,
    ) -> Result<crate::webrtc::SessionDescription, String> {
        webrtc::create_webrtc_offer(peer_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Create WebRTC answer
    pub async fn create_webrtc_answer(
        &self,
        peer_id: String,
    ) -> Result<crate::webrtc::SessionDescription, String> {
        webrtc::create_webrtc_answer(peer_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Set remote description
    pub async fn set_remote_description(
        &self,
        peer_id: String,
        description: crate::webrtc::SessionDescription,
    ) -> Result<String, String> {
        webrtc::set_remote_description(peer_id, description).await
    }

    #[cfg(feature = "webrtc")]
    /// Add ICE candidate
    pub async fn add_ice_candidate(
        &self,
        peer_id: String,
        candidate: crate::webrtc::IceCandidate,
    ) -> Result<String, String> {
        webrtc::add_ice_candidate(peer_id, candidate).await
    }

    #[cfg(feature = "webrtc")]
    /// Get local ICE candidates
    pub async fn get_local_ice_candidates(
        &self,
        peer_id: String,
    ) -> Result<Vec<crate::webrtc::IceCandidate>, String> {
        webrtc::get_local_ice_candidates(peer_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Add video transceivers
    pub async fn add_video_transceivers(
        &self,
        peer_id: String,
        layers: Vec<crate::webrtc::streaming::SimulcastLayer>,
    ) -> Result<String, String> {
        webrtc::add_video_transceivers(peer_id, layers).await
    }

    #[cfg(feature = "webrtc")]
    /// Create data channel
    pub async fn create_data_channel(
        &self,
        peer_id: String,
        channel_label: String,
    ) -> Result<String, String> {
        webrtc::create_data_channel(peer_id, channel_label).await
    }

    #[cfg(feature = "webrtc")]
    /// Send data channel message
    pub async fn send_data_channel_message(
        &self,
        peer_id: String,
        channel_label: String,
        message: Vec<u8>,
    ) -> Result<String, String> {
        webrtc::send_data_channel_message(peer_id, channel_label, message).await
    }

    #[cfg(feature = "webrtc")]
    /// Get peer connection status
    pub async fn get_peer_connection_status(
        &self,
        peer_id: String,
    ) -> Result<crate::webrtc::peer::PeerConnectionStats, String> {
        webrtc::get_peer_connection_status(peer_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Close peer connection
    pub async fn close_peer_connection(&self, peer_id: String) -> Result<String, String> {
        webrtc::close_peer_connection(peer_id).await
    }

    #[cfg(feature = "webrtc")]
    /// List peer connections
    pub async fn list_peer_connections(
        &self,
    ) -> Result<Vec<crate::webrtc::peer::PeerConnectionStats>, String> {
        webrtc::list_peer_connections().await
    }

    #[cfg(feature = "webrtc")]
    /// Get WebRTC system status
    pub async fn get_webrtc_system_status(&self) -> Result<WebRTCSystemStatus, String> {
        webrtc::get_webrtc_system_status().await
    }

    #[cfg(feature = "webrtc")]
    /// Pause WebRTC stream
    pub async fn pause_webrtc_stream(&self, stream_id: String) -> Result<String, String> {
        webrtc::pause_webrtc_stream(stream_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Resume WebRTC stream
    pub async fn resume_webrtc_stream(&self, stream_id: String) -> Result<String, String> {
        webrtc::resume_webrtc_stream(stream_id).await
    }

    #[cfg(feature = "webrtc")]
    /// Set WebRTC stream bitrate
    pub async fn set_webrtc_stream_bitrate(
        &self,
        stream_id: String,
        bitrate: u32,
    ) -> Result<String, String> {
        webrtc::set_webrtc_stream_bitrate(stream_id, bitrate).await
    }

    #[cfg(feature = "webrtc")]
    pub async fn start_webrtc_streaming(
        &self,
        device_id: String,
        stream_id: String,
        _config: Option<crate::webrtc::StreamConfig>,
        mode: Option<crate::webrtc::StreamMode>,
        callback: Option<Box<dyn Fn(CameraFrame) + Send + Sync>>,
    ) -> Result<String, String> {
        webrtc::start_webrtc_streaming(device_id, stream_id, _config, mode, callback).await
    }
}
