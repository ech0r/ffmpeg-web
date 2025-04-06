use web_sys::{Event, HtmlSelectElement, HtmlInputElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct CodecSelectorProps {
    pub video_codec: String,
    pub audio_codec: String,
    pub video_bitrate: String,
    pub audio_bitrate: String,
    pub on_video_codec_change: Callback<String>,
    pub on_audio_codec_change: Callback<String>,
    pub on_video_bitrate_change: Callback<String>,
    pub on_audio_bitrate_change: Callback<String>,
    #[prop_or(false)]
    pub disabled: bool,
}

#[function_component(CodecSelector)]
pub fn codec_selector(props: &CodecSelectorProps) -> Html {
    let on_video_codec_change = {
        let callback = props.on_video_codec_change.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            callback.emit(select.value());
        })
    };

    let on_audio_codec_change = {
        let callback = props.on_audio_codec_change.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            callback.emit(select.value());
        })
    };

    let on_video_bitrate_change = {
        let callback = props.on_video_bitrate_change.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            callback.emit(input.value());
        })
    };

    let on_audio_bitrate_change = {
        let callback = props.on_audio_bitrate_change.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            callback.emit(input.value());
        })
    };

    html! {
        <>
            <div class="form-group">
                <label for="video-codec">{ "Video Codec:" }</label>
                <select 
                    id="video-codec"
                    value={props.video_codec.clone()}
                    onchange={on_video_codec_change}
                    disabled={props.disabled}
                >
                    <option value="h264">{ "H.264 (AVC)" }</option>
                    <option value="h265">{ "H.265 (HEVC)" }</option>
                    <option value="vp8">{ "VP8" }</option>
                    <option value="vp9">{ "VP9" }</option>
                    <option value="av1">{ "AV1" }</option>
                    <option value="copy">{ "Copy (no re-encode)" }</option>
                </select>
            </div>

            <div class="form-group">
                <label for="video-bitrate">{ "Video Bitrate (kbps):" }</label>
                <input
                    id="video-bitrate"
                    type="number"
                    min="100"
                    max="20000"
                    step="100"
                    value={props.video_bitrate.clone()}
                    onchange={on_video_bitrate_change}
                    disabled={props.disabled || props.video_codec == "copy"}
                />
            </div>

            <div class="form-group">
                <label for="audio-codec">{ "Audio Codec:" }</label>
                <select 
                    id="audio-codec"
                    value={props.audio_codec.clone()}
                    onchange={on_audio_codec_change}
                    disabled={props.disabled}
                >
                    <option value="aac">{ "AAC" }</option>
                    <option value="mp3">{ "MP3" }</option>
                    <option value="opus">{ "Opus" }</option>
                    <option value="flac">{ "FLAC" }</option>
                    <option value="copy">{ "Copy (no re-encode)" }</option>
                </select>
            </div>

            <div class="form-group">
                <label for="audio-bitrate">{ "Audio Bitrate (kbps):" }</label>
                <input
                    id="audio-bitrate"
                    type="number"
                    min="32"
                    max="320"
                    step="16"
                    value={props.audio_bitrate.clone()}
                    onchange={on_audio_bitrate_change}
                    disabled={props.disabled || props.audio_codec == "copy"}
                />
            </div>
        </>
    }
}
