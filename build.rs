fn main() {
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");

    // 把 dylib 输出路径嵌入到编译产物中
    let out_dir = std::env::var("OUT_DIR").unwrap();
    // OUT_DIR 类似 target/release/build/mediaremote-rs-xxx/out
    // dylib 在 target/release/libmediaremote_rs.dylib
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();
    let dylib_path = target_dir.join("libmediaremote_rs.dylib");

    println!("cargo:rustc-env=MEDIAREMOTE_DYLIB_PATH={}", dylib_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}
