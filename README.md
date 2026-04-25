# mediaremote-rs

[![Crates.io](https://img.shields.io/crates/v/mediaremote-rs?style=flat-square)](https://crates.io/crates/mediaremote-rs)
[![License](https://img.shields.io/crates/l/mediaremote-rs?style=flat-square)](https://opensource.org/licenses/MIT)

**mediaremote-rs** 是一个 Rust 库，用于访问 macOS 的私有 `MediaRemote.framework`。它允许开发者获取系统当前的媒体播放信息（如曲目、艺术家、进度等），并监听播放状态的实时变化。

由于 macOS 15.4+ 对 MediaRemote 框架实施了更严格的访问限制，本库通过独特的跨进程适配器技术，确保在最新版 macOS 上依然可用。

---

## ✨ 特性

- **获取播放信息**：读取当前曲目标题、艺术家、专辑、时长、播放进度等。
- **实时状态监听**：订阅媒体状态变化，仅在发生变动时接收更新（基于长生命周期适配器进程和 Rust Channel）。
- **专辑封面支持**：获取二进制封面图片及 MIME 类型。
- **权限检测**：提供 API 检测当前环境是否支持 MediaRemote 访问。
- **强类型接口**：提供完整的 Rust 类型定义和 JSON 序列化支持。
- **广泛兼容性**：支持 Apple Music, Spotify, Chrome, IINA 等所有集成系统媒体控制的应用。

## 📦 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
mediaremote-rs = "0.2.0"
```

## 🚀 使用方法

在 Rust 项目中使用本库非常简单，只需在 `Cargo.toml` 中添加依赖后，即可通过以下方式调用：

### 1. 基础查询

获取当前的播放状态和详细信息。

```rust
use mediaremote_rs::{get_now_playing, is_playing, test_access};

fn main() {
    // 可选：检查是否能够访问 MediaRemote 服务
    if !test_access() {
        eprintln!("无法访问 MediaRemote 服务，请检查权限或系统版本。");
        return;
    }

    // 检查是否有媒体正在播放
    if is_playing() {
        println!("正在播放中...");
    }

    // 获取详细信息
    if let Some(info) = get_now_playing() {
        println!("标题: {}", info.title);
        println!("艺术家: {}", info.artist.unwrap_or_default());
        println!("专辑: {}", info.album.unwrap_or_default());
        
        if let Some(duration) = info.duration {
            println!("时长: {:.1} 秒", duration);
        }
    } else {
        println!("当前没有媒体播放信息");
    }
}
```

### 2. 实时监听

使用 `subscribe` 函数创建一个监听器，它会在后台线程轮询并在状态变化时发送消息。

```rust
use mediaremote_rs::subscribe;
use std::time::Duration;

fn main() {
    // 每 500ms 检查一次变化
    let receiver = subscribe(Duration::from_millis(500));

    println!("开始监听媒体状态变化 (按 Ctrl+C 停止)...");

    for info in receiver {
        if info.playing {
            println!("▶️ {} - {}", info.title, info.artist.unwrap_or("未知艺术家".into()));
        } else {
            println!("⏸️ 已暂停: {}", info.title);
        }
    }
}
```

## 🧩 数据结构

核心结构体 `NowPlayingInfo` 包含了所有可用的媒体信息：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingInfo {
    pub bundle_identifier: String,           // 应用包 ID (如 "com.apple.Music")
    pub playing: bool,                       // 播放状态
    pub title: String,                       // 标题
    pub artist: Option<String>,              // 艺术家
    pub album: Option<String>,               // 专辑
    pub duration: Option<f64>,               // 总时长(秒)
    pub elapsed_time: Option<f64>,           // 当前进度(秒)
    pub artwork_mime_type: Option<String>,   // 封面格式 (如 "image/jpeg")
    pub artwork_data: Option<Vec<u8>>,       // 封面二进制数据
    pub playback_rate: Option<f64>,          // 播放速率
}
```

## 🛠️ 工作原理与架构

### 背景
从 macOS 15.4 开始，苹果限制了普通应用直接加载 `MediaRemote.framework`。只有拥有特定 entitlement 或 `com.apple.` 前缀的系统应用才能正常使用。

### 解决方案
本库采用了一种**双进程架构**来绕过此限制：

1.  **Rust 主进程**：你的应用程序。
2.  **Perl 适配器**：利用系统内置的 `/usr/bin/perl` (它拥有 `com.apple.perl5` 签名，因此有权访问 MediaRemote)。

### 流程
1.  Rust 程序在运行时提取一个预编译的动态库 (`libmediaremote_rs.dylib`) 到临时目录。
2.  通过 `Command` 调用系统 Perl，加载该动态库。
3.  动态库通过 Objective-C 接口调用 MediaRemote API 获取数据。
4.  数据序列化为 JSON 并通过标准输出返回给 Rust 主进程。

这种方法既保证了功能的可用性，又通过 Rust 封装提供了类型安全和易用性。

### 致谢
本库的实现思路和后续维护思路均来自于 [ungive/mediaremote-adapter](https://github.com/ungive/mediaremote-adapter)。

## ⚠️ 注意事项

- **性能**：单次查询会启动短生命周期适配器进程；`subscribe` 模式会复用一个长生命周期适配器进程，避免每次刷新都重复启动 Perl。
- **环境要求**：
    - macOS 10.12+
    - 系统必须安装有 `/usr/bin/perl` (macOS 默认自带)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License
