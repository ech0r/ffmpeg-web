use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FileInputProps {
    pub on_file_selected: Callback<web_sys::File>,
    #[prop_or(false)]
    pub disabled: bool,
}

#[function_component(FileInput)]
pub fn file_input(props: &FileInputProps) -> Html {
    let on_change = {
        let on_file_selected = props.on_file_selected.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    on_file_selected.emit(file);
                }
            }
        })
    };

    html! {
        <div class="file-input">
            <label for="file-upload" class="file-label">
                { "Select Video File" }
            </label>
            <input 
                id="file-upload"
                type="file" 
                accept="video/*,audio/*"
                onchange={on_change}
                disabled={props.disabled}
            />
        </div>
    }
}
