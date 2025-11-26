# mediaremote-rs

Access macOS MediaRemote.framework to get now playing information.

Works on **macOS 15.4+** by using `/usr/bin/perl` as a loader to bypass permission restrictions.

## Usage

```toml
[target.'cfg(target_os = "macos")'.dependencies]
mediaremote-rs = "0.1"
```

```rust
use std::time::Duration;

// Get playback state
let playing = mediaremote_rs::is_playing();

// Get now playing info
if let Some(info) = mediaremote_rs::get_now_playing() {
    println!("{} - {}", info.title, info.artist.unwrap_or_default());
}

// Subscribe to changes
let rx = mediaremote_rs::subscribe(Duration::from_millis(500));
for info in rx {
    println!("Now playing: {}", info.title);
}
```

## API

- `is_playing() -> bool` - Check if media is currently playing
- `get_now_playing() -> Option<NowPlayingInfo>` - Get current playback info
- `subscribe(interval: Duration) -> Receiver<NowPlayingInfo>` - Subscribe to playback changes

## NowPlayingInfo

```rust
pub struct NowPlayingInfo {
    pub bundle_identifier: String,
    pub playing: bool,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub elapsed_time: Option<f64>,
    pub artwork_mime_type: Option<String>,
    pub artwork_data: Option<String>,  // base64 encoded
    pub playback_rate: Option<f64>,
}
```

## License

MIT

## Acknowledgments

Inspired by and built upon the implementation from [mediaremote-adapter](https://github.com/ungive/mediaremote-adapter).