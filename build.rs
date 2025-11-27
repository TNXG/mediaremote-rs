use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/libmediaremote_rs.dylib");
    
    // 使用 CARGO_MANIFEST_DIR 确保路径正确
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let resources_dir = manifest_dir.join("resources");
    
    // 确保 resources 目录存在
    if !resources_dir.exists() {
        fs::create_dir_all(&resources_dir).expect("Failed to create resources directory");
    }
    
    // 如果 dylib 不存在，创建一个占位文件以允许首次编译
    // CI 流程会在首次编译后用真正的 dylib 替换它，然后重新编译
    let dylib_path = resources_dir.join("libmediaremote_rs.dylib");
    if !dylib_path.exists() {
        // 创建一个最小的占位文件
        fs::write(&dylib_path, &[]).expect("Failed to create placeholder dylib");
        println!("cargo:warning=Created placeholder dylib at {:?}. Run build again after copying the real dylib.", dylib_path);
    }
}
