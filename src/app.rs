use crate::components::{
    file_input::FileInput,
    format_selector::FormatSelector,
    codec_selector::CodecSelector,
    progress::Progress,
};
use crate::ffmpeg;
use yew::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{Url, HtmlAnchorElement, File, FileReader};
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use gloo::console::log;

pub struct App {
    input_file: Option<File>,
    input_data: Option<Vec<u8>>,
    output_format: String,
    video_codec: String,
    audio_codec: String,
    video_bitrate: String,
    audio_bitrate: String,
    resolution: String,
    custom_resolution: String,
    transcoding: bool,
    progress: f64,
    logs: Vec<String>,
    processed_data: Option<Vec<u8>>,
    download_ready: bool,
}

pub enum Msg {
    FileSelected(File),
    FileLoaded(Vec<u8>),
    SetOutputFormat(String),
    SetVideoCodec(String),
    SetAudioCodec(String),
    SetVideoBitrate(String),
    SetAudioBitrate(String),
    SetResolution(String),
    SetCustomResolution(String),
    StartTranscoding,
    TranscodingProgress(f64),
    AddLog(String),
    TranscodingFinished(Vec<u8>),
    TranscodingError(String),
    DownloadFile,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        // Initialize FFmpeg
        ffmpeg::init_ffmpeg();
        
        Self {
            input_file: None,
            input_data: None,
            output_format: "mp4".to_string(),
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
            video_bitrate: "1000".to_string(),
            audio_bitrate: "128".to_string(),
            resolution: "same".to_string(),
            custom_resolution: "1280x720".to_string(),
            transcoding: false,
            progress: 0.0,
            logs: vec!["Welcome to FFmpeg WebAssembly Transcoder".to_string()],
            processed_data: None,
            download_ready: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FileSelected(file) => {
                let file_name = file.name();
                self.input_file = Some(file.clone());
                self.add_log(ctx, format!("File selected: {}", file_name));
                self.download_ready = false;
                self.processed_data = None;
                
                // Read file contents
                let file_reader = FileReader::new().unwrap();
                let file_reader_clone = file_reader.clone();
                let link = ctx.link().clone();
                
                let onload = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    let array_buffer = file_reader_clone.result().unwrap();
                    let uint8array = js_sys::Uint8Array::new(&array_buffer);
                    let mut buffer = vec![0; uint8array.length() as usize];
                    uint8array.copy_to(&mut buffer[..]);
                    link.send_message(Msg::FileLoaded(buffer));
                }) as Box<dyn FnMut(web_sys::Event)>);
                
                file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                
                file_reader.read_as_array_buffer(&file).unwrap();
                true
            },
            Msg::FileLoaded(data) => {
                self.input_data = Some(data);
                self.add_log(ctx, format!("File loaded: {} bytes", self.input_data.as_ref().unwrap().len()));
                true
            },
            Msg::SetOutputFormat(format) => {
                self.output_format = format;
                true
            },
            Msg::SetVideoCodec(codec) => {
                self.video_codec = codec;
                true
            },
            Msg::SetAudioCodec(codec) => {
                self.audio_codec = codec;
                true
            },
            Msg::SetVideoBitrate(bitrate) => {
                self.video_bitrate = bitrate;
                true
            },
            Msg::SetAudioBitrate(bitrate) => {
                self.audio_bitrate = bitrate;
                true
            },
            Msg::SetResolution(res) => {
                self.resolution = res;
                true
            },
            Msg::SetCustomResolution(res) => {
                self.custom_resolution = res;
                true
            },
            Msg::StartTranscoding => {
                if self.input_data.is_none() {
                    self.add_log(ctx, "Error: No input file data available".to_string());
                    return true;
                }
                
                self.transcoding = true;
                self.progress = 0.0;
                self.download_ready = false;
                self.processed_data = None;
                self.add_log(ctx, "Starting transcoding process...".to_string());
                
                // Get transcoding parameters
                let input_data = self.input_data.as_ref().unwrap().clone();
                let output_format = self.output_format.clone();
                let video_codec = self.video_codec.clone();
                let audio_codec = self.audio_codec.clone();
                let video_bitrate = self.video_bitrate.parse::<i32>().unwrap_or(1000);
                let audio_bitrate = self.audio_bitrate.parse::<i32>().unwrap_or(128);
                let resolution = if self.resolution == "custom" {
                    self.custom_resolution.clone()
                } else {
                    self.resolution.clone()
                };
                
                // Clone link for async context
                let link = ctx.link().clone();
                
                // Spawn transcoding task
                wasm_bindgen_futures::spawn_local(async move {
                    link.send_message(Msg::AddLog(format!(
                        "Transcoding to {} format with {}({}) and {}({}) codecs",
                        output_format, video_codec, video_bitrate, audio_codec, audio_bitrate
                    )));
                    
                    // Progress simulation for UI feedback
                    for i in 1..=9 {
                        let progress = (i as f64) * 10.0;
                        let link_clone = link.clone();
                        
                        // Use a simple callback for progress updates
                        let callback = gloo::timers::callback::Timeout::new(100, move || {
                            link_clone.send_message(Msg::TranscodingProgress(progress));
                        });
                        callback.forget();
                        
                        // Wait a bit before next update
                        gloo::timers::callback::Timeout::new(100 * i, move || {}).forget();
                    }
                    
                    // Perform actual transcoding
                    link.send_message(Msg::AddLog("Processing file with FFmpeg...".to_string()));
                    
                    match ffmpeg::transcode(
                        &input_data,
                        &output_format,
                        &video_codec,
                        &audio_codec,
                        video_bitrate,
                        audio_bitrate,
                        &resolution,
                    ) {
                        Ok(output_data) => {
                            link.send_message(Msg::TranscodingProgress(100.0));
                            link.send_message(Msg::AddLog(format!("Transcoding completed! Output size: {} bytes", output_data.len())));
                            link.send_message(Msg::TranscodingFinished(output_data));
                        },
                        Err(error) => {
                            link.send_message(Msg::TranscodingError(error.to_string()));
                        }
                    }
                });
                
                true
            },
            Msg::TranscodingProgress(prog) => {
                self.progress = prog;
                true
            },
            Msg::AddLog(log) => {
                self.logs.push(log);
                true
            },
            Msg::TranscodingFinished(data) => {
                self.transcoding = false;
                self.processed_data = Some(data);
                self.download_ready = true;
                log("Setting download_ready to true");
                self.add_log(ctx, "Transcoding finished successfully! Click 'Download' to save your file.".to_string());
                true
            },
            Msg::TranscodingError(error) => {
                self.transcoding = false;
                self.add_log(ctx, format!("Error during transcoding: {}", error));
                true
            },
            Msg::DownloadFile => {
                if let Some(data) = &self.processed_data {
                    log("Download button clicked, file data available");
                    let window = web_sys::window().expect("no global window exists");
                    let document = window.document().expect("no document exists");
                    
                    // Create a Uint8Array from our data
                    let uint8arr = Uint8Array::new_with_length(data.len() as u32);
                    for (i, &byte) in data.iter().enumerate() {
                        uint8arr.set_index(i as u32, byte);
                    }
                    
                    // Convert Uint8Array to Blob
                    let array = js_sys::Array::new();
                    array.push(&uint8arr.buffer());
                    
                    let mut blob_options = web_sys::BlobPropertyBag::new();
                    
                    // Set MIME type based on output format
                    let mime_type = match self.output_format.as_str() {
                        "mp4" => "video/mp4",
                        "webm" => "video/webm",
                        "mkv" => "video/x-matroska",
                        "mov" => "video/quicktime",
                        "gif" => "image/gif",
                        "mp3" => "audio/mpeg",
                        "ogg" => "audio/ogg",
                        "wav" => "audio/wav",
                        _ => "application/octet-stream",
                    };
                    
                    blob_options.set_type(mime_type);
                    
                    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
                        &array, &blob_options).unwrap();
                    
                    // Create a download URL
                    let url = Url::create_object_url_with_blob(&blob).unwrap();
                    
                    // Create a download link
                    let a = document
                        .create_element("a")
                        .unwrap()
                        .dyn_into::<HtmlAnchorElement>()
                        .unwrap();
                    
                    // Generate output filename
                    let filename = match &self.input_file {
                        Some(file) => {
                            let name = file.name();
                            if let Some(dot_pos) = name.rfind('.') {
                                format!("{}.{}", &name[0..dot_pos], self.output_format)
                            } else {
                                format!("output.{}", self.output_format)
                            }
                        },
                        None => format!("output.{}", self.output_format),
                    };
                    
                    a.set_href(&url);
                    a.set_download(&filename);
                    a.set_attribute("style", "display: none").unwrap();
                    
                    document.body().unwrap().append_child(&a).unwrap();
                    a.click();
                    document.body().unwrap().remove_child(&a).unwrap();
                    
                    Url::revoke_object_url(&url).unwrap();
                    
                    self.add_log(ctx, format!("File '{}' downloaded", filename));
                } else {
                    log("No processed data available");
                    self.add_log(ctx, "No processed file available to download".to_string());
                }
                
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // Debug statement outside of HTML
        if self.download_ready {
            log("Rendering download button because download_ready is true");
        }
        
        html! {
            <div class="app-container">
                <header>
                    <h1>{ "FFmpeg WebAssembly Transcoder" }</h1>
                </header>
                
                <main>
                    <div class="panel">
                        <h2>{ "Input" }</h2>
                        <FileInput 
                            on_file_selected={ctx.link().callback(Msg::FileSelected)}
                            disabled={self.transcoding}
                        />
                        
                        <h2>{ "Output Settings" }</h2>
                        <div class="settings-grid">
                            <FormatSelector 
                                format={self.output_format.clone()}
                                on_format_change={ctx.link().callback(Msg::SetOutputFormat)}
                                disabled={self.transcoding}
                            />
                            
                            <CodecSelector 
                                video_codec={self.video_codec.clone()}
                                audio_codec={self.audio_codec.clone()}
                                video_bitrate={self.video_bitrate.clone()}
                                audio_bitrate={self.audio_bitrate.clone()}
                                on_video_codec_change={ctx.link().callback(Msg::SetVideoCodec)}
                                on_audio_codec_change={ctx.link().callback(Msg::SetAudioCodec)}
                                on_video_bitrate_change={ctx.link().callback(Msg::SetVideoBitrate)}
                                on_audio_bitrate_change={ctx.link().callback(Msg::SetAudioBitrate)}
                                disabled={self.transcoding}
                            />
                            
                            <div class="form-group">
                                <label for="resolution">{ "Resolution:" }</label>
                                <select 
                                    id="resolution"
                                    value={self.resolution.clone()}
                                    onchange={ctx.link().callback(|e: Event| {
                                        let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                        Msg::SetResolution(target.value())
                                    })}
                                    disabled={self.transcoding}
                                >
                                    <option value="same">{ "Same as source" }</option>
                                    <option value="720p">{ "720p (1280x720)" }</option>
                                    <option value="1080p">{ "1080p (1920x1080)" }</option>
                                    <option value="custom">{ "Custom" }</option>
                                </select>
                                
                                {
                                    if self.resolution == "custom" {
                                        html! {
                                            <input 
                                                type="text"
                                                value={self.custom_resolution.clone()}
                                                onchange={ctx.link().callback(|e: Event| {
                                                    let target: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                    Msg::SetCustomResolution(target.value())
                                                })}
                                                placeholder="width x height"
                                                disabled={self.transcoding}
                                            />
                                        }
                                    } else {
                                        html! {}
                                    }
                                }
                            </div>
                        </div>
                        
                        <div class="button-container">
                            <button 
                                onclick={ctx.link().callback(|_| Msg::StartTranscoding)}
                                disabled={self.input_file.is_none() || self.transcoding}
                                class="transcode-button"
                            >
                                { "Start Transcoding" }
                            </button>
                            
                            // Download button - using download_ready flag explicitly
                            {
                                if self.download_ready {
                                    html! {
                                        <button 
                                            onclick={ctx.link().callback(|_| Msg::DownloadFile)}
                                            class="download-button"
                                        >
                                            { "Download File" }
                                        </button>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                        </div>
                    </div>
                    
                    <div class="panel">
                        <h2>{ "Progress" }</h2>
                        <Progress value={self.progress} />
                        
                        <h2>{ "Logs" }</h2>
                        <div class="logs-container">
                            {
                                for self.logs.iter().map(|log| {
                                    html! { <div class="log-entry">{ log }</div> }
                                })
                            }
                        </div>
                    </div>
                </main>
            </div>
        }
    }
}

impl App {
    fn add_log(&mut self, ctx: &Context<Self>, message: String) {
        let timestamp = js_sys::Date::new_0().to_locale_time_string("en-US");
        let log_entry = format!("[{}] {}", timestamp, message);
        ctx.link().send_message(Msg::AddLog(log_entry));
    }
}
