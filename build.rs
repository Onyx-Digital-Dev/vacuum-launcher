use std::env;

fn main() {
    // Declare the custom cfg attributes
    println!("cargo:rustc-check-cfg=cfg(cava_enabled)");
    println!("cargo:rustc-check-cfg=cfg(cava_disabled)");
    
    // Only build cava integration if explicitly requested via feature flag
    if cfg!(feature = "cava") {
        build_cava_integration();
    } else {
        println!("cargo:rustc-cfg=cava_disabled");
    }
}

fn build_cava_integration() {
    use std::path::PathBuf;
    use std::process::Command;

    let cava_dir = PathBuf::from("cava");
    
    if !cava_dir.exists() {
        println!("cargo:warning=Cava source directory not found at ./cava/");
        println!("cargo:warning=To enable Cava integration:");
        println!("cargo:warning=1. git clone https://github.com/karlstav/cava.git");
        println!("cargo:warning=2. cargo build --features cava");
        println!("cargo:rustc-cfg=cava_disabled");
        return;
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
    let cava_build_dir = PathBuf::from(&out_dir).join("cava_build");
    
    // Create build directory
    if let Err(e) = std::fs::create_dir_all(&cava_build_dir) {
        println!("cargo:warning=Failed to create Cava build directory: {}", e);
        println!("cargo:rustc-cfg=cava_disabled");
        return;
    }
    
    // Configure with cmake - manually specify paths for NixOS
    let cmake_status = Command::new("cmake")
        .current_dir(&cava_build_dir)
        .arg("../../../cava")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DBUILD_SHARED_LIBS=ON")
        .arg("-DCMAKE_PREFIX_PATH=/run/current-system/sw")
        .arg("-DFFTW3_INCLUDE_DIR=/run/current-system/sw/include")
        .arg("-DFFTW3_LIBRARY=/run/current-system/sw/lib/libfftw3.so")
        .status();
    
    if cmake_status.is_err() || !cmake_status.unwrap().success() {
        println!("cargo:warning=CMake configuration failed for Cava");
        println!("cargo:warning=Ensure cmake and development libraries are installed");
        println!("cargo:rustc-cfg=cava_disabled");
        return;
    }
    
    // Build the library
    let build_status = Command::new("cmake")
        .current_dir(&cava_build_dir)
        .arg("--build")
        .arg(".")
        .arg("--target")
        .arg("cavacore")
        .status();
    
    if build_status.is_err() || !build_status.unwrap().success() {
        println!("cargo:warning=Cava library build failed");
        println!("cargo:rustc-cfg=cava_disabled");
        return;
    }

    // Tell cargo where to find the library
    println!("cargo:rustc-link-search=native={}", cava_build_dir.display());
    println!("cargo:rustc-link-lib=cavacore");
    
    // Link against required dependencies
    println!("cargo:rustc-link-lib=fftw3");
    println!("cargo:rustc-link-lib=m");
    
    // Generate bindings
    if let Err(e) = generate_bindings() {
        println!("cargo:warning=Failed to generate Cava bindings: {}", e);
        println!("cargo:rustc-cfg=cava_disabled");
        return;
    }
    
    println!("cargo:rustc-cfg=cava_enabled");
    println!("cargo:rerun-if-changed=cava/");
}

fn generate_bindings() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    let bindings = bindgen::Builder::default()
        .header("cava/src/cavacore.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()?;

    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("cava_bindings.rs"))?;
    
    Ok(())
}