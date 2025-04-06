use web_sys::{Event, HtmlSelectElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FormatSelectorProps {
    pub format: String,
    pub on_format_change: Callback<String>,
    #[prop_or(false)]
    pub disabled: bool,
}

#[function_component(FormatSelector)]
pub fn format_selector(props: &FormatSelectorProps) -> Html {
    let on_change = {
        let on_format_change = props.on_format_change.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            on_format_change.emit(select.value());
        })
    };

    html! {
        <div class="form-group">
            <label for="output-format">{ "Output Format:" }</label>
            <select 
                id="output-format"
                value={props.format.clone()}
                onchange={on_change}
                disabled={props.disabled}
            >
                <option value="mp4">{ "MP4" }</option>
                <option value="webm">{ "WebM" }</option>
                <option value="mkv">{ "MKV" }</option>
                <option value="mov">{ "MOV" }</option>
                <option value="gif">{ "GIF" }</option>
                <option value="mp3">{ "MP3 (audio only)" }</option>
                <option value="ogg">{ "OGG (audio only)" }</option>
                <option value="wav">{ "WAV (audio only)" }</option>
            </select>
        </div>
    }
}
