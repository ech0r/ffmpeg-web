# FFmpeg WebAssembly Transcoder

A browser-based video transcoder that uses FFmpeg compiled to WebAssembly, built entirely in Rust with the Yew framework.

## Overview

This project provides a pure WebAssembly implementation of video transcoding using FFmpeg. Unlike other solutions that rely on JavaScript bridges, this project compiles FFmpeg directly to WebAssembly and creates direct Rust bindings to it.

## Features

- Upload video files directly in your browser
- Convert between various formats (MP4, WebM, MKV, etc.)
- Change video/audio codecs (H.264, H.265, VP9, etc.)
- Adjust bitrate and resolution
- Download processed videos directly in the browser
- Real-time progress tracking
- No server-side processing required

## Requirements

To build this project, you'll need:

1. [Rust](https://www.rust-lang.org/tools/install) with the wasm32 target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):
   ```bash
   cargo install wasm-pack
   ```

3. [Emscripten SDK](https://emscripten.org/docs/getting_started/downloads.html):
   ```bash
   git clone https://github.com/emscripten-core/emsdk.git
   cd emsdk
   ./emsdk install latest
   ./emsdk activate latest
   source ./emsdk_env.sh  # or emsdk_env.bat on Windows
   ```

## Building the Project

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/ffmpeg-transcoder.git
   cd ffmpeg-transcoder
   ```

2. Build the project:
   ```bash
   wasm-pack build --target web
   ```

   **Note**: The first build will take a significant amount of time as it compiles FFmpeg from source. Subsequent builds will be much faster.

3. Serve the application:
   ```bash
   # Create a dist directory
   mkdir -p dist
   
   # Copy necessary files
   cp -r static/* dist/
   cp -r pkg dist/
   
   # Serve with your preferred web server
   python3 -m http.server --directory dist 8080
   ```

4. Open your browser to `http://localhost:8080`

## How It Works

1. **FFmpeg Compilation**: During the build process, FFmpeg is compiled to WebAssembly using Emscripten. This is handled by the `build.rs` script.

2. **C Wrapper**: A C wrapper (`ffmpeg_wrapper.c`) is used to create a simplified API around FFmpeg's complex codebase.

3. **Rust Bindings**: The Rust code (`src/ffmpeg.rs`) creates direct bindings to the compiled WebAssembly module.

4. **UI Layer**: The user interface is built with Yew, a Rust framework for creating web applications.

## Project Structure

- `build.rs` - Build script that compiles FFmpeg to WebAssembly
- `src/`
  - `lib.rs` - Main Rust library entry point
  - `app.rs` - Yew application component
  - `ffmpeg.rs` - Rust bindings to FFmpeg WebAssembly
  - `ffmpeg_wrapper.c` - C wrapper around FFmpeg libraries
  - `ffmpeg_pre.js` - JavaScript utilities for FFmpeg WebAssembly
  - `components/` - UI components
- `static/` - Static files (HTML, CSS)

## Memory Management

Video processing can be memory-intensive. This implementation:

1. Uses streaming where possible to reduce memory usage
2. Sets initial WebAssembly memory to 32MB
3. Allows memory growth up to 512MB
4. Frees memory as soon as it's no longer needed

## Browser Compatibility

This application works in browsers that support WebAssembly and the necessary Web APIs:

- Chrome/Edge (version 79+)
- Firefox (version 72+)
- Safari (version 13.1+)

Mobile browsers may have more limited memory and performance.

## Limitations

- Maximum file size depends on browser memory limitations
- Some codecs might not be available due to patent restrictions
- Complex video processing operations might be slow compared to native applications

## License

This project is licensed under MIT License - see the LICENSE file for details.

## Acknowledgments

- FFmpeg team for their incredible video processing library
- Rust and Yew teams for making WebAssembly development possible
- Emscripten project for the WebAssembly compiler infrastructure
