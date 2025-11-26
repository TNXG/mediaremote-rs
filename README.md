# mediaremote-rs

[![Crates.io](https://img.shields.io/crates/v/mediaremote-rs)](https://crates.io/crates/mediaremote-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

这是一个 Rust 库，用于访问 macOS 的 `MediaRemote.framework`，以便从媒体应用程序中检索“正在播放”的信息。该库允许开发者获取当前曲目信息、检查播放状态，并在媒体状态发生变化时流式传输实时更新。

## 特性

- **“正在播放”信息**：获取当前曲目详情，包括标题、艺术家、专辑、时长和播放进度。
- **实时更新**：以可自定义的时间间隔流式传输媒体状态的实时变化。
- **专辑封面**：以 Base64 编码字符串形式访问专辑封面数据，支持自动检测 MIME 类型。
- **跨进程支持**：提供基于 Perl 的适配器，用于与外部进程集成。
- **类型安全**：完整的 Rust 类型系统，支持 JSON 序列化。
- **macOS 原生**：直接绑定 Apple 的 MediaRemote.framework 以获得最佳性能。

## 系统要求

- **macOS** (推荐 10.12 或更高版本)
- **Rust** 1.85+ (Edition 2024)
- **Xcode 命令行工具** (用于框架链接)

## 使用方法

### 基本用法

```rust
use mediaremote_rs::{get_now_playing, is_playing};

// 检查媒体当前是否正在播放
if is_playing() {
    println!("Something is playing!");
}

// 获取当前播放信息
if let Some(info) = get_now_playing() {
    println!("Now playing: {} - {}", info.title, info.artist.unwrap_or("Unknown".to_string()));
}
```

### 实时更新

```rust
use std::time::Duration;
use mediaremote_rs::subscribe;

// 订阅实时更新，每 500ms 检查一次
let receiver = subscribe(Duration::from_millis(500));

for info in receiver {
    if info.playing {
        println!("🎵 {} - {}", info.title, info.artist.unwrap_or("Unknown".to_string()));
    } else {
        println!("⏸️ Paused: {}", info.title);
    }
}
```

### 完整示例

```rust
use mediaremote_rs::NowPlayingInfo;
use std::time::Duration;

fn main() {
    println!("macOS Media Remote Example");

    // 简单检查
    if mediaremote_rs::is_playing() {
        println!("Media is currently playing");
    }

    // 获取详细信息
    if let Some(info) = mediaremote_rs::get_now_playing() {
        print_now_playing_info(&info);
    }

    // 实时监控
    println!("\nStarting real-time monitoring (Ctrl+C to exit)...");
    let receiver = mediaremote_rs::subscribe(Duration::from_secs(1));

    for info in receiver {
        print_now_playing_info(&info);
    }
}

fn print_now_playing_info(info: &NowPlayingInfo) {
    println!("\n🎵 Now Playing:");
    println!("  App: {}", info.bundle_identifier);
    println!("  Title: {}", info.title);

    if let Some(artist) = &info.artist {
        println!("  Artist: {}", artist);
    }

    if let Some(album) = &info.album {
        println!("  Album: {}", album);
    }

    if let Some(duration) = info.duration {
        if let Some(elapsed) = info.elapsed_time {
            println!("  Progress: {:.1}s / {:.1}s", elapsed, duration);
        } else {
            println!("  Duration: {:.1}s", duration);
        }
    }

    if let Some(artwork) = &info.artwork_data {
        println!("  Artwork: {} ({} bytes)",
                info.artwork_mime_type.as_ref().unwrap_or(&"unknown".to_string()),
                artwork.len());
    }

    println!("  Status: {}", if info.playing { "▶️ Playing" } else { "⏸️ Paused" });
}
```

## 数据结构

该库提供了一个全面的 `NowPlayingInfo` 结构体：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingInfo {
    pub bundle_identifier: String,           // 应用包 ID (例如 "com.apple.Music")
    pub playing: bool,                       // 当前播放状态
    pub title: String,                       // 曲目标题
    pub artist: Option<String>,              // 艺术家名称
    pub album: Option<String>,               // 专辑名称
    pub duration: Option<f64>,               // 总时长（秒）
    pub elapsed_time: Option<f64>,           // 当前进度（秒）
    pub artwork_mime_type: Option<String>,   // 封面 MIME 类型 (JPEG, PNG 等)
    pub artwork_data: Option<String>,        // Base64 编码的封面数据
    pub playback_rate: Option<f64>,          // 播放速率 (1.0 = 正常速度)
}
```

## 支持的应用程序

此库适用于任何使用标准媒体远程框架 (Media Remote Framework) 的 macOS 应用程序，包括：

- **Apple Music**
- **Spotify**
- **VLC**
- **QuickTime Player**
- **Safari** (用于网页视频/音频)
- **Chrome**, **Firefox** (用于网页媒体)
- 还有更多...

### 构建输出

- **Debug**: `target/debug/libmediaremote_rs.dylib`
- **Release**: `target/release/libmediaremote_rs.dylib`

该库会构建为以下两种形式：
- **cdylib**: 供其他语言使用的动态链接库
- **rlib**: 供 Rust 直接集成的 Rust 库

## 工作原理

### 为什么这个方案可行

根据技术发现，具有 `com.apple.` 前缀的应用程序包标识符被系统授予访问 MediaRemote 框架的权限。Perl 平台二进制文件 `/usr/bin/perl` 被系统认可为拥有 `com.apple.perl5` 的包标识符。

你可以通过在运行时使用 Console.app 流式传输日志消息来验证这一点：

```
default	14:44:55.871495+0200	mediaremoted	Adding client <MRDMediaRemoteClient 0x15820b1a0, bundleIdentifier = com.apple.perl5, pid = 86889>
```

通过利用这个系统特性，本库能够绕过现代 macOS 版本对直接 MediaRemote 访问的限制。

## 架构

该库采用了精密的多层架构：

1.  **核心层 (Core Layer)**: 与 `MediaRemote.framework` 的直接 Objective-C 绑定。
2.  **适配层 (Adapter Layer)**: 使用 `DynaLoader` 的基于 Perl 的跨进程兼容层，绕过系统权限限制。
3.  **API 层 (API Layer)**: 清晰、类型安全的 Rust 接口。
4.  **流式层 (Streaming Layer)**: 使用 Rust Channel 实现的实时更新。

## 错误处理

库提供了优雅的错误处理机制：

```rust
// 所有函数都返回 Option<T> 以处理以下情况：
// - 无媒体正在播放
// - 应用程序不支持媒体远程控制
// - 系统权限阻止访问
// - 框架调用失败

let info = get_now_playing();
match info {
    Some(data) => println!("Got info: {}", data.title),
    None => println!("No media currently playing"),
}
```

## 环境变量

- `MEDIAREMOTE_DYLIB_PATH`: 覆盖编译后的 dylib 文件路径。
- `MEDIAREMOTE_DYLIB_PATH` 在构建过程中会自动嵌入，用于运行时解析。

## 线程安全

- 所有公共函数都是线程安全的。
- `subscribe()` 函数会生成一个专用的监控线程。
- 共享状态使用 Rust 的所有权系统进行保护。

## 性能

- **最小开销**：通过 Perl 适配器的优化调用，避免频繁的进程切换开销。
- **高效流式传输**：仅在发生实际更改时才发送更新。
- **低内存占用**：自动清理 Objective-C 对象。
- **快速启动**：无初始化延迟或预热期。

## 项目背景

### 创建动机

从 macOS 15.4 开始，MediaRemote 框架在应用程序中直接加载时完全失效。尽管有许多相关的问题报告，但苹果公司尚未提供官方解决方案。

本项目旨在：
1. 提供一个功能完整的替代方案，让开发者能够持续访问媒体播放信息
2. 激励苹果为我们提供一个公共 API，用于读取媒体播放信息和控制设备上的媒体播放
3. 为 Rust 生态系统贡献一个高质量的媒体控制库

## 许可证

本项目采用 MIT 许可证 - 详情请参阅 [LICENSE](LICENSE) 文件。

## 贡献

欢迎贡献代码！请随时提交 Pull Request。对于重大更改，请先开一个 Issue 讨论您想要更改的内容。

## 致谢

本项目灵感来源于 [mediaremote-adapter](https://github.com/ungive/mediaremote-adapter) 的实现，该项目首次发现了 Perl 适配器方案来解决 MediaRemote 框架的访问限制问题。我们在此基础上用 Rust 重新实现，提供了一个类型安全、高性能的库接口，更便于 Rust 开发者集成和使用。
