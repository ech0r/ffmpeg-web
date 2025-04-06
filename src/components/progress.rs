use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProgressProps {
    pub value: f64,
}

#[function_component(Progress)]
pub fn progress(props: &ProgressProps) -> Html {
    html! {
        <div class="progress-container">
            <div class="progress-bar" style={format!("width: {}%", props.value)}></div>
            <div class="progress-text">{ format!("{}%", props.value) }</div>
        </div>
    }
}
