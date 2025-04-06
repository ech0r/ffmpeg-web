use wasm_bindgen::prelude::*;
use std::fmt;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    // FFmpeg WebAssembly module
    type FFmpegModule;
    
    #[wasm_bindgen(js_namespace = window)]
    fn FFmpeg() -> FFmpegModule;
    
    #[wasm_bindgen(method, js_name = "setProgressCallback")]
    fn set_progress_callback(this: &FFmpegModule, callback: &Closure<dyn FnMut(f32)>);
    
    #[wasm_bindgen(method, js_name = "transcodeAsync")]
    fn transcode_async(
        this: &FFmpegModule,
        input_data: &[u8],
        output_format: &str,
        video_codec: &str,
        audio_codec: &str,
        video_bitrate: i32,
        audio_bitrate: i32,
        resolution: &str
    ) -> js_sys::Promise;
    
    #[wasm_bindgen(method, catch, js_name = "_init_ffmpeg")]
    fn init_ffmpeg_js(this: &FFmpegModule) -> Result<(), JsValue>;
}

#[derive(Debug)]
pub struct TranscodeError(String);

impl fmt::Display for TranscodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transcoding error: {}", self.0)
    }
}

impl std::error::Error for TranscodeError {}

// Convert JsValue to TranscodeError
impl From<JsValue> for TranscodeError {
    fn from(value: JsValue) -> Self {
        let error_msg = if let Some(err) = value.as_string() {
            err
        } else if let Some(err) = js_sys::Error::from(value).message().as_string() {
            err
        } else {
            "Unknown error".to_string()
        };
        
        TranscodeError(error_msg)
    }
}

// Static callback closure to avoid leaking memory
thread_local! {
    static PROGRESS_CALLBACK: std::cell::RefCell<Option<Closure<dyn FnMut(f32)>>> = std::cell::RefCell::new(None);
}

/// Initialize the FFmpeg WebAssembly module
pub fn init_ffmpeg() {
    match FFmpeg().init_ffmpeg_js() {
        Ok(_) => log("FFmpeg WebAssembly module initialized successfully"),
        Err(e) => log(&format!("Failed to initialize FFmpeg: {:?}", e)),
    }
}

/// Set a progress callback for transcoding operations
pub fn set_progress_handler<F>(callback: F) 
where 
    F: FnMut(f32) + 'static
{
    PROGRESS_CALLBACK.with(|cell| {
        let new_callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut(f32)>);
        FFmpeg().set_progress_callback(&new_callback);
        *cell.borrow_mut() = Some(new_callback);
    });
}

/// Transcode a media file
pub async fn transcode(
    input_data: &[u8],
    output_format: &str,
    video_codec: &str,
    audio_codec: &str,
    video_bitrate: i32,
    audio_bitrate: i32,
    resolution: &str,
) -> Result<Vec<u8>, TranscodeError> {
    // Create a promise for the transcoding operation
    let promise = FFmpeg().transcode_async(
        input_data,
        output_format,
        video_codec,
        audio_codec,
        video_bitrate,
        audio_bitrate,
        resolution
    );
    
    // Convert the promise to a Rust future
    let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
    
    // Convert the JavaScript Uint8Array to a Rust Vec<u8>
    let uint8array = js_sys::Uint8Array::new(&result);
    let mut output_data = vec![0; uint8array.length() as usize];
    uint8array.copy_to(&mut output_data);
    
    Ok(output_data)
}

/// Get the available video codecs
pub fn get_video_codecs() -> Vec<String> {
    // These are the codecs we support in our build
    vec![
        "h264".to_string(),
        "h265".to_string(),
        "vp8".to_string(),
        "vp9".to_string(),
        "av1".to_string(),
        "mpeg4".to_string(),
    ]
}

/// Get the available audio codecs
pub fn get_audio_codecs() -> Vec<String> {
    // These are the codecs we support in our build
    vec![
        "aac".to_string(),
        "mp3".to_string(),
        "opus".to_string(),
        "vorbis".to_string(),
        "flac".to_string(),
    ]
}

/// Get the available output formats
pub fn get_output_formats() -> Vec<String> {
    // These are the formats we support in our build
    vec![
        "mp4".to_string(),
        "webm".to_string(),
        "mkv".to_string(),
        "mov".to_string(),
        "avi".to_string(),
        "mp3".to_string(),
        "ogg".to_string(),
        "wav".to_string(),
    ]
}
