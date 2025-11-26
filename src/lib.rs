#![cfg(target_os = "macos")]

mod mediaremote;
mod types;

pub use types::NowPlayingInfo;

use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

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

fn get_dylib_path() -> Option<String> {
    // 优先使用编译时嵌入的路径
    let compile_time_path = option_env!("MEDIAREMOTE_DYLIB_PATH");
    if let Some(p) = compile_time_path {
        let path = std::path::Path::new(p);
        if path.exists() {
            return Some(p.to_string());
        }
    }

    // 回退：运行时查找
    let candidates = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("libmediaremote_rs.dylib"))),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("deps/libmediaremote_rs.dylib"))),
        std::env::var("MEDIAREMOTE_DYLIB_PATH")
            .ok()
            .map(std::path::PathBuf::from),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return candidate.to_str().map(|s| s.to_string());
        }
    }
    None
}

fn call_adapter() -> Option<NowPlayingInfo> {
    let dylib_path = get_dylib_path()?;
    let output = Command::new("/usr/bin/perl")
        .arg("-e")
        .arg(LOADER_SCRIPT)
        .arg(&dylib_path)
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
