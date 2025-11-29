use crate::types::NowPlayingInfo;
use block2::StackBlock;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::NSString;
use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;
use std::time::Duration;

#[link(name = "MediaRemote", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn MRMediaRemoteGetNowPlayingInfo(dispatch_queue: *const c_void, callback: *const c_void);
    fn MRMediaRemoteGetNowPlayingClient(dispatch_queue: *const c_void, callback: *const c_void);
}

#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    fn dispatch_queue_create(label: *const i8, attr: *const c_void) -> *const c_void;
}

/// Get now playing information from MediaRemote
pub fn get_now_playing_info() -> Option<NowPlayingInfo> {
    let result: Cell<Option<NowPlayingInfo>> = Cell::new(None);
    let bundle_id: Cell<Option<String>> = Cell::new(None);

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

    unsafe {
        let queue =
            dispatch_queue_create(b"com.mediaremote.queue\0".as_ptr() as *const i8, ptr::null());

        MRMediaRemoteGetNowPlayingClient(queue, (&*bundle_block) as *const _ as *const c_void);
        MRMediaRemoteGetNowPlayingInfo(queue, (&*block) as *const _ as *const c_void);
    }

    // Run the run loop briefly to allow the callback to execute
    run_loop_once(Duration::from_millis(500));

    // Merge bundle_id into result
    if let Some(mut info) = result.take() {
        if let Some(bid) = bundle_id.take() {
            info.bundle_identifier = bid;
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

    // Determine if playing based on playback rate
    info.playing = info.playback_rate.map(|r| r > 0.0).unwrap_or(false);

    // Get artwork data
    let artwork_key = NSString::from_str("kMRMediaRemoteNowPlayingInfoArtworkData");
    let artwork_data: *const AnyObject = msg_send![dict, objectForKey: &*artwork_key];
    if !artwork_data.is_null() {
        let bytes: *const c_void = msg_send![artwork_data, bytes];
        let len: usize = msg_send![artwork_data, length];
        if !bytes.is_null() && len > 0 {
            let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, len) };
            info.artwork_data = Some(base64_encode(slice));
            info.artwork_mime_type = detect_image_mime_type(slice);
        }
    }

    Some(info)
}

pub fn test_access() -> bool {
    get_now_playing_info().is_some()
}

fn run_loop_once(duration: Duration) {
    unsafe {
        let run_loop: *const AnyObject = msg_send![class!(NSRunLoop), currentRunLoop];
        let date: *const AnyObject =
            msg_send![class!(NSDate), dateWithTimeIntervalSinceNow: duration.as_secs_f64()];
        let _: () = msg_send![run_loop, runUntilDate: date];
    }
}

/// Base64 encode bytes
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
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
        && &data[8..12] == &[0x57, 0x45, 0x42, 0x50]
    {
        return Some("image/webp".to_string());
    }

    if data.starts_with(&[0x42, 0x4D]) {
        return Some("image/bmp".to_string());
    }

    None
}
