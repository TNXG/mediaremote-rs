use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingInfo {
    pub bundle_identifier: String,
    pub playing: bool,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_rate: Option<f64>,
}

impl Default for NowPlayingInfo {
    fn default() -> Self {
        Self {
            bundle_identifier: String::new(),
            playing: false,
            title: String::new(),
            artist: None,
            album: None,
            duration: None,
            elapsed_time: None,
            artwork_mime_type: None,
            artwork_data: None,
            playback_rate: None,
        }
    }
}
