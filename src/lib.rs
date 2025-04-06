use wasm_bindgen::prelude::*;

mod app;
mod components;
mod ffmpeg;

#[wasm_bindgen(start)]
pub fn run_app() {
    // Initialize console error panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Initialize the app
    yew::Renderer::<app::App>::new().render();
}
