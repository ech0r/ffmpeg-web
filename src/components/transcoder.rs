use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TranscoderProps {
    pub start_transcoding: Callback<()>,
    pub disabled: bool,
}

#[function_component(Transcoder)]
pub fn transcoder(props: &TranscoderProps) -> Html {
    let onclick = {
        let callback = props.start_transcoding.clone();
        Callback::from(move |_| {
            callback.emit(());
        })
    };

    html! {
        <button 
            onclick={onclick}
            disabled={props.disabled}
            class="transcode-button"
        >
            { "Start Transcoding" }
        </button>
    }
}
