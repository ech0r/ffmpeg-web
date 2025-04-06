use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only build WebAssembly when targeting wasm32
    if env::var("TARGET").unwrap().contains("wasm32") {
        let out_dir = env::var("OUT_DIR").unwrap();
        let out_path = Path::new(&out_dir);
        let ffmpeg_dir = out_path.join("ffmpeg");
        
        // Check for Emscripten
        let emcc_check = Command::new("emcc")
            .arg("--version")
            .output();
            
        if emcc_check.is_err() {
            panic!("Emscripten compiler (emcc) not found. Please install Emscripten SDK first.");
        }
        
        // Clone FFmpeg if not already done
        if !ffmpeg_dir.exists() {
            println!("cargo:warning=Cloning FFmpeg repository...");
            let _ = Command::new("git")
                .args(&["clone", "--depth", "1", "https://git.ffmpeg.org/ffmpeg.git"])
                .current_dir(&out_path)
                .status()
                .expect("Failed to clone FFmpeg repository");
        } else {
            println!("cargo:warning=FFmpeg repository already exists. Skipping clone.");
        }
        
        // Configure FFmpeg for WebAssembly compilation
        if !ffmpeg_dir.join(".configured").exists() {
            println!("cargo:warning=Configuring FFmpeg for WebAssembly...");
            let _ = Command::new("emconfigure")
                .current_dir(&ffmpeg_dir)
                .arg("./configure")
                .args(&[
                    "--target-os=none",
                    "--arch=x86_32",
                    "--enable-cross-compile",
                    "--disable-x86asm",
                    "--disable-inline-asm",
                    "--disable-stripping",
                    "--disable-programs",
                    "--disable-doc",
                    "--disable-avdevice",
                    "--disable-postproc",
                    "--disable-avfilter",
                    "--disable-network",
                    "--disable-iconv",
                    "--enable-small",
                    &format!("--prefix={}", ffmpeg_dir.join("build").display())
                ])
                .status()
                .expect("Failed to configure FFmpeg");
                
            // Create .configured marker file
            std::fs::write(ffmpeg_dir.join(".configured"), "").unwrap();
        } else {
            println!("cargo:warning=FFmpeg already configured. Skipping configuration.");
        }
        
        // Build FFmpeg
        if !ffmpeg_dir.join(".built").exists() {
            println!("cargo:warning=Building FFmpeg (this may take a while)...");
            let _ = Command::new("emmake")
                .current_dir(&ffmpeg_dir)
                .arg("make")
                .arg("-j4")
                .status()
                .expect("Failed to build FFmpeg");
                
            // Create .built marker file
            std::fs::write(ffmpeg_dir.join(".built"), "").unwrap();
        } else {
            println!("cargo:warning=FFmpeg already built. Skipping build.");
        }
        
        // Install FFmpeg to the build directory
        if !ffmpeg_dir.join("build").exists() {
            println!("cargo:warning=Installing FFmpeg...");
            let _ = Command::new("emmake")
                .current_dir(&ffmpeg_dir)
                .arg("make")
                .arg("install")
                .status()
                .expect("Failed to install FFmpeg");
        } else {
            println!("cargo:warning=FFmpeg already installed. Skipping installation.");
        }
        
        // Compile our C wrapper
        println!("cargo:warning=Compiling FFmpeg wrapper...");
        let wrapper_c = concat!(env!("CARGO_MANIFEST_DIR"), "/src/ffmpeg_wrapper.c");
        let wrapper_js = out_path.join("ffmpeg_wrapper.js");
        let wrapper_wasm = out_path.join("ffmpeg_wrapper.wasm");
        
        let _ = Command::new("emcc")
            .args(&[
                wrapper_c,
                "-o", &wrapper_js.to_string_lossy(),
                "-I", &ffmpeg_dir.join("build/include").to_string_lossy(),
                "-L", &ffmpeg_dir.join("build/lib").to_string_lossy(),
                "-lavformat", "-lavcodec", "-lswscale", "-lavutil", "-lswresample",
                "-s", "WASM=1",
                "-s", "ALLOW_MEMORY_GROWTH=1",
                "-s", "INITIAL_MEMORY=33554432", // 32MB initial heap
                "-s", "MAXIMUM_MEMORY=536870912", // 512MB max heap
                "-s", "EXPORTED_FUNCTIONS=['_malloc','_free','_init_ffmpeg','_transcode','_free_transcode_result']",
                "-s", "EXPORTED_RUNTIME_METHODS=['ccall','cwrap']",
                "-O3",  // Optimization level
                "--pre-js", concat!(env!("CARGO_MANIFEST_DIR"), "/src/ffmpeg_pre.js"),
            ])
            .status()
            .expect("Failed to compile FFmpeg wrapper");
            
        // Copy generated files to where they'll be accessible
        let target_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
        std::fs::create_dir_all(target_dir).unwrap();
        
        std::fs::copy(
            wrapper_js, 
            Path::new(target_dir).join("ffmpeg_wrapper.js")
        ).expect("Failed to copy ffmpeg_wrapper.js");
        
        std::fs::copy(
            wrapper_wasm, 
            Path::new(target_dir).join("ffmpeg_wrapper.wasm")
        ).expect("Failed to copy ffmpeg_wrapper.wasm");
        
        println!("cargo:rustc-link-search=native={}", ffmpeg_dir.join("build/lib").display());
        println!("cargo:rerun-if-changed=src/ffmpeg_wrapper.c");
        println!("cargo:rerun-if-changed=src/ffmpeg_pre.js");
    }
}
