use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/libmediaremote_rs.dylib");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let resources_dir = manifest_dir.join("resources");

    if !resources_dir.exists() {
        fs::create_dir_all(&resources_dir).expect("Failed to create resources directory");
    }

    let dylib_path = resources_dir.join("libmediaremote_rs.dylib");
    if !dylib_path.exists() {
        fs::write(&dylib_path, &[]).expect("Failed to create placeholder dylib");
        println!(
            "cargo:warning=Created placeholder dylib at {:?}. Run build again after copying the real dylib.",
            dylib_path
        );
    } else if dylib_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        sign_dylib(&dylib_path);
    }
}

fn sign_dylib(dylib_path: &PathBuf) {
    let output = Command::new("codesign")
        .arg("--force")
        .arg("--deep")
        .arg("-s")
        .arg("-")
        .arg(dylib_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                println!("cargo:warning=Successfully signed dylib with ad-hoc signature");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("cargo:warning=Failed to sign dylib: {}", stderr);
            }
        }
        Err(e) => {
            // codesign 不存在或失败不是致命错误
            println!("cargo:warning=Could not run codesign: {}", e);
        }
    }
}
