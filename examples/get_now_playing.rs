use mediaremote_rs::{get_now_playing, test_access};

fn main() {
    if !test_access() {
        eprintln!("MediaRemote adapter is not accessible.");
        std::process::exit(1);
    }

    let Some(info) = get_now_playing() else {
        println!("No now playing metadata is available.");
        return;
    };

    println!("Bundle: {}", info.bundle_identifier);
    println!("Playing: {}", info.playing);
    println!("Title: {}", info.title);

    if let Some(artist) = info.artist {
        println!("Artist: {artist}");
    }

    if let Some(album) = info.album {
        println!("Album: {album}");
    }

    if let Some(duration) = info.duration {
        println!("Duration: {duration:.3}s");
    }

    if let Some(elapsed_time) = info.elapsed_time {
        println!("Elapsed: {elapsed_time:.3}s");
    }

    if let Some(playback_rate) = info.playback_rate {
        println!("Playback rate: {playback_rate}");
    }

    if let Some(artwork_data) = info.artwork_data {
        let mime_type = info.artwork_mime_type.as_deref().unwrap_or("unknown");
        println!("Artwork: {} bytes ({mime_type})", artwork_data.len());
    }
}
