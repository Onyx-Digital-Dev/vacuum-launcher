use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Only build cavacore if we have the source
    let cava_dir = PathBuf::from("cava");
    
    if !cava_dir.exists() {
        println!("cargo:warning=Cava source not found. Run: git clone https://github.com/karlstav/cava.git");
        println!("cargo:warning=Audio visualizer will be disabled.");
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let cava_build_dir = PathBuf::from(&out_dir).join("cava_build");
    
    // Create build directory
    std::fs::create_dir_all(&cava_build_dir).unwrap();
    
    // Run cmake to configure
    let cmake_status = Command::new("cmake")
        .current_dir(&cava_build_dir)
        .arg("../../../cava")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DBUILD_SHARED_LIBS=ON")
        .status();
    
    if cmake_status.is_err() || !cmake_status.unwrap().success() {
        println!("cargo:warning=CMake configuration failed. Audio visualizer disabled.");
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
        println!("cargo:warning=Cava build failed. Audio visualizer disabled.");
        return;
    }

    // Tell cargo where to find the library
    println!("cargo:rustc-link-search=native={}", cava_build_dir.display());
    println!("cargo:rustc-link-lib=cavacore");
    
    // Link against required dependencies
    println!("cargo:rustc-link-lib=fftw3");
    println!("cargo:rustc-link-lib=m");
    
    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("cava/src/cavacore.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("cava_bindings.rs"))
        .expect("Couldn't write bindings!");
        
    println!("cargo:rerun-if-changed=cava/");
}