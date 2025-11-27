#![cfg(target_os = "macos")]

mod mediaremote;
mod types;

pub use types::NowPlayingInfo;

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

/// 嵌入预编译的 dylib
const EMBEDDED_DYLIB: &[u8] = include_bytes!("../resources/libmediaremote_rs.dylib");

/// Perl 加载脚本
const LOADER_SCRIPT: &str = r#"
use strict;
use warnings;
use DynaLoader;
my $dylib_path = shift @ARGV or exit 1;
my $command = shift @ARGV // "get";
exit 1 unless -e $dylib_path;
my $handle = DynaLoader::dl_load_file($dylib_path, 0) or exit 1;
my $symbol_name = $command eq "test" ? "adapter_test" : "adapter_get_env";
my $symbol = DynaLoader::dl_find_symbol($handle, $symbol_name) or exit 1;
DynaLoader::dl_install_xsub("main::run", $symbol);
eval { main::run(); };
exit($@ ? 1 : 0);
"#;

/// 获取 dylib 路径（首次调用时提取到临时目录）
fn get_dylib_path() -> Option<&'static str> {
    static DYLIB_PATH: OnceLock<Option<String>> = OnceLock::new();
    
    DYLIB_PATH.get_or_init(|| {
        // 使用固定的临时目录路径，避免重复提取
        let cache_dir = std::env::temp_dir().join("mediaremote-rs");
        let dylib_path = cache_dir.join("libmediaremote_rs.dylib");
        
        // 检查是否需要重新提取（文件不存在或大小不匹配）
        let need_extract = match fs::metadata(&dylib_path) {
            Ok(meta) => meta.len() != EMBEDDED_DYLIB.len() as u64,
            Err(_) => true,
        };
        
        if need_extract {
            // 创建目录
            if fs::create_dir_all(&cache_dir).is_err() {
                return None;
            }
            
            // 写入 dylib
            let mut file = fs::File::create(&dylib_path).ok()?;
            file.write_all(EMBEDDED_DYLIB).ok()?;
            
            // 设置可执行权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&dylib_path).ok()?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&dylib_path, perms).ok()?;
            }
        }
        
        dylib_path.to_str().map(|s| s.to_string())
    }).as_deref()
}

fn call_adapter() -> Option<NowPlayingInfo> {
    let dylib_path = get_dylib_path()?;
    
    let output = Command::new("/usr/bin/perl")
        .arg("-e")
        .arg(LOADER_SCRIPT)
        .arg(dylib_path)
        .arg("get")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_str = stdout.trim();

    if json_str == "null" || json_str.is_empty() {
        return None;
    }

    serde_json::from_str(json_str).ok()
}

/// 获取当前播放状态
pub fn is_playing() -> bool {
    call_adapter().map(|info| info.playing).unwrap_or(false)
}

/// 获取当前播放信息
pub fn get_now_playing() -> Option<NowPlayingInfo> {
    call_adapter()
}

/// 流式订阅播放信息变化
pub fn subscribe(interval: Duration) -> Receiver<NowPlayingInfo> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut last: Option<NowPlayingInfo> = None;

        loop {
            if let Some(info) = call_adapter() {
                let changed = match &last {
                    None => true,
                    Some(prev) => {
                        prev.title != info.title
                            || prev.artist != info.artist
                            || prev.playing != info.playing
                    }
                };

                if changed {
                    if tx.send(info.clone()).is_err() {
                        break;
                    }
                    last = Some(info);
                }
            } else if last.is_some() {
                last = None;
            }

            thread::sleep(interval);
        }
    });

    rx
}

/// 测试是否可以访问 MediaRemote
pub fn test_access() -> bool {
    if let Some(dylib_path) = get_dylib_path() {
        let output = Command::new("/usr/bin/perl")
            .arg("-e")
            .arg(LOADER_SCRIPT)
            .arg(dylib_path)
            .arg("test")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        
        output.map(|s| s.success()).unwrap_or(false)
    } else {
        false
    }
}

// FFI exports (called by Perl)

#[unsafe(no_mangle)]
pub extern "C" fn adapter_get_env() {
    let result = mediaremote::get_now_playing_info();
    match result {
        Some(info) => {
            if let Ok(json) = serde_json::to_string(&info) {
                println!("{}", json);
            } else {
                println!("null");
            }
        }
        None => println!("null"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn adapter_test() {
    if mediaremote::get_now_playing_info().is_some() || mediaremote::test_access() {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}
