use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 为 PrivateFrameworks 设置 Framework 搜索路径（方案 2：weak_framework 需要）
    // 方案 1（dlopen）是在运行时通过完整路径加载 MediaRemote，因此这里并非必须，
    // 但为兼容可能引用的其他框架，仍保留该设置。
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");

    // 方案 2：weak_framework 链接参数
    // 不使用 #[link(name = "MediaRemote")]（这会创建强依赖），
    // 而是通过 -weak_framework 告诉链接器：从 MediaRemote 解析符号，但不要求
    // 所有传递依赖（例如 SwiftUICore）在链接时必须存在。
    // 如果希望使用方案 2 而不是 dlopen（方案 1），可以取消下面两行的注释：
    //
    // println!("cargo:rustc-cdylib-link-arg=-weak_framework");
    // println!("cargo:rustc-cdylib-link-arg=MediaRemote");

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
        println!(
            "cargo:warning=Created placeholder dylib at {:?}. Run build again after copying the real dylib.",
            dylib_path
        );
    } else if dylib_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        // 如果 dylib 存在且有内容，尝试进行代码签名
        sign_dylib(&dylib_path);
    }
}

fn sign_dylib(dylib_path: &PathBuf) {
    // 在 macOS 上对 dylib 进行代码签名
    // 使用 - (ad-hoc签名) 可以在没有证书的情况下进行签名
    // 这样可以避免 macOS 弹出安全警告
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
