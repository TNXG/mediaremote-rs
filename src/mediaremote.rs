use crate::types::NowPlayingInfo;
use block2::StackBlock;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::NSString;
use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;
use std::sync::OnceLock;
use std::time::Duration;

type MRGetNowPlayingInfoFn = unsafe extern "C" fn(*const c_void, *const c_void);
type MRGetNowPlayingClientFn = unsafe extern "C" fn(*const c_void, *const c_void);
type MRGetNowPlayingApplicationIsPlayingFn = unsafe extern "C" fn(*const c_void, *const c_void);

/// Dynamically loaded MediaRemote framework bindings.
///
/// Uses dlopen/dlsym to avoid link-time dependency on MediaRemote.framework,
/// which transitively pulls in SwiftUICore and causes Xcode 16 linker errors.
struct MediaRemoteBindings {
    get_now_playing_info: MRGetNowPlayingInfoFn,
    get_now_playing_client: MRGetNowPlayingClientFn,
    get_now_playing_application_is_playing: MRGetNowPlayingApplicationIsPlayingFn,
}

static BINDINGS: OnceLock<Option<MediaRemoteBindings>> = OnceLock::new();

/// Load MediaRemote.framework at runtime via dlopen/dlsym.
fn get_bindings() -> Option<&'static MediaRemoteBindings> {
    BINDINGS
        .get_or_init(|| unsafe {
            let path = b"/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote\0";
            let handle = libc::dlopen(path.as_ptr() as *const i8, libc::RTLD_LAZY);
            if handle.is_null() {
                return None;
            }

            let get_info = libc::dlsym(
                handle,
                b"MRMediaRemoteGetNowPlayingInfo\0".as_ptr() as *const i8,
            );
            let get_client = libc::dlsym(
                handle,
                b"MRMediaRemoteGetNowPlayingClient\0".as_ptr() as *const i8,
            );
            let get_is_playing = libc::dlsym(
                handle,
                b"MRMediaRemoteGetNowPlayingApplicationIsPlaying\0".as_ptr() as *const i8,
            );

            if get_info.is_null() || get_client.is_null() || get_is_playing.is_null() {
                return None;
            }

            Some(MediaRemoteBindings {
                get_now_playing_info: std::mem::transmute(get_info),
                get_now_playing_client: std::mem::transmute(get_client),
                get_now_playing_application_is_playing: std::mem::transmute(get_is_playing),
            })
        })
        .as_ref()
}

#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    fn dispatch_queue_create(label: *const i8, attr: *const c_void) -> *const c_void;
}

/// Get now playing information from MediaRemote
pub fn get_now_playing_info() -> Option<NowPlayingInfo> {
    let bindings = get_bindings()?;
    let result: Cell<Option<NowPlayingInfo>> = Cell::new(None);
    let bundle_id: Cell<Option<String>> = Cell::new(None);
    let playing: Cell<Option<bool>> = Cell::new(None);

    // Get bundle identifier from MRMediaRemoteGetNowPlayingClient
    let bundle_block = StackBlock::new(|client: *const AnyObject| {
        if !client.is_null() {
            unsafe {
                let bid: *const AnyObject = msg_send![client, bundleIdentifier];
                if !bid.is_null() {
                    let ns_str: *const NSString = bid as *const NSString;
                    bundle_id.set(Some((*ns_str).to_string()));
                }
            }
        }
    });

    let block = StackBlock::new(|info_dict: *const AnyObject| {
        if !info_dict.is_null() {
            let info = unsafe { parse_now_playing_dict(info_dict) };
            result.set(info);
        }
    });

    let playing_block = StackBlock::new(|is_playing: i8| {
        playing.set(Some(is_playing != 0));
    });

    unsafe {
        let queue = dispatch_queue_create(
            b"com.mediaremote.queue\0".as_ptr() as *const i8,
            ptr::null(),
        );

        (bindings.get_now_playing_client)(queue, (&*bundle_block) as *const _ as *const c_void);
        (bindings.get_now_playing_info)(queue, (&*block) as *const _ as *const c_void);
        (bindings.get_now_playing_application_is_playing)(
            queue,
            (&*playing_block) as *const _ as *const c_void,
        );
    }

    // Run the run loop briefly to allow the callback to execute
    run_loop_once(Duration::from_millis(500));

    // Merge bundle_id into result
    if let Some(mut info) = result.take() {
        if let Some(bid) = bundle_id.take() {
            info.bundle_identifier = bid;
        }
        if let Some(is_playing) = playing.take() {
            info.playing = is_playing;
        }
        Some(info)
    } else {
        None
    }
}

/// Parse the NSDictionary returned by MediaRemote into our struct
unsafe fn parse_now_playing_dict(dict_ptr: *const AnyObject) -> Option<NowPlayingInfo> {
    if dict_ptr.is_null() {
        return None;
    }

    let dict = unsafe { &*dict_ptr };
    let mut info = NowPlayingInfo::default();

    // Helper to get string value from dict
    let get_string = |dict: &AnyObject, key: &str| -> Option<String> {
        let key_ns = NSString::from_str(key);
        let value: *const AnyObject = msg_send![dict, objectForKey: &*key_ns];
        if value.is_null() {
            return None;
        }
        let desc: *const NSString = msg_send![value, description];
        if desc.is_null() {
            return None;
        }
        Some(unsafe { (*desc).to_string() })
    };

    // Helper to get number value from dict
    let get_number = |dict: &AnyObject, key: &str| -> Option<f64> {
        let key_ns = NSString::from_str(key);
        let value: *const AnyObject = msg_send![dict, objectForKey: &*key_ns];
        if value.is_null() {
            return None;
        }
        Some(msg_send![value, doubleValue])
    };

    // Parse title (required)
    info.title = get_string(dict, "kMRMediaRemoteNowPlayingInfoTitle").unwrap_or_default();
    if info.title.is_empty() {
        return None;
    }

    // Parse optional fields
    info.artist = get_string(dict, "kMRMediaRemoteNowPlayingInfoArtist");
    info.album = get_string(dict, "kMRMediaRemoteNowPlayingInfoAlbum");
    info.duration = get_number(dict, "kMRMediaRemoteNowPlayingInfoDuration");
    info.elapsed_time = get_number(dict, "kMRMediaRemoteNowPlayingInfoElapsedTime");
    info.playback_rate = get_number(dict, "kMRMediaRemoteNowPlayingInfoPlaybackRate");

    // 仅作为兜底：优先使用 MediaRemote 专用播放状态回调。
    info.playing = info.playback_rate.map(|r| r > 0.0).unwrap_or(false);

    // Get artwork data
    let artwork_key = NSString::from_str("kMRMediaRemoteNowPlayingInfoArtworkData");
    let artwork_data: *const AnyObject = msg_send![dict, objectForKey: &*artwork_key];
    if !artwork_data.is_null() {
        let bytes: *const c_void = msg_send![artwork_data, bytes];
        let len: usize = msg_send![artwork_data, length];
        if !bytes.is_null() && len > 0 {
            let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, len) };
            info.artwork_data = Some(slice.to_vec());
            info.artwork_mime_type = detect_image_mime_type(slice);
        }
    }

    Some(info)
}

pub fn test_access() -> bool {
    get_bindings().is_some()
}

fn run_loop_once(duration: Duration) {
    unsafe {
        let run_loop: *const AnyObject = msg_send![class!(NSRunLoop), currentRunLoop];
        let date: *const AnyObject =
            msg_send![class!(NSDate), dateWithTimeIntervalSinceNow: duration.as_secs_f64()];
        let _: () = msg_send![run_loop, runUntilDate: date];
    }
}

/// Detect image MIME type from magic bytes
fn detect_image_mime_type(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }

    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg".to_string());
    }

    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png".to_string());
    }

    if data.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
        return Some("image/gif".to_string());
    }

    if data.len() >= 12
        && data.starts_with(&[0x52, 0x49, 0x46, 0x46])
        && data[8..12] == [0x57, 0x45, 0x42, 0x50]
    {
        return Some("image/webp".to_string());
    }

    if data.starts_with(&[0x42, 0x4D]) {
        return Some("image/bmp".to_string());
    }

    None
}
