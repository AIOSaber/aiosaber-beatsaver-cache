use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatSaverMap {
    pub id: String,
    pub name: String,
    pub description: String,
    // uploader
    pub metadata: MapMetadata,
    // stats
    pub automapper: bool,
    pub ranked: bool,
    pub qualified: bool,
    pub versions: Vec<MapVersion>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_published_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapMetadata {
    pub bpm: f32,
    pub duration: u32,
    pub song_name: String,
    pub song_sub_name: String,
    pub song_author_name: String,
    pub level_author_name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapVersion {
    pub hash: String,
    pub key: String,
    // = id
    pub state: MapVersionState,
    pub created_at: DateTime<Utc>,
    // sageScore
    pub diffs: Vec<MapDifficulty>,
    #[serde(rename(serialize = "downloadURL", deserialize = "downloadURL"))]
    pub download_url: String,
    #[serde(rename(serialize = "coverURL", deserialize = "coverURL"))]
    pub cover_url: String,
    #[serde(rename(serialize = "previewURL", deserialize = "previewURL"))]
    pub preview_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum MapVersionState {
    Published,
    Uploaded,
    Testplay,
    Feedback,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapDifficulty {
    pub bombs: u32,
    pub characteristic: DifficultyCharacteristic,
    pub chroma: bool,
    pub cinema: bool,
    pub difficulty: DifficultyType,
    pub events: u32,
    pub me: bool,
    pub ne: bool,
    pub njs: f32,
    pub notes: u32,
    pub nps: f64,
    pub obstacles: u32,
    pub offset: f32,
    pub seconds: f64,
    pub stars: Option<f32>,
    pub parity_summary: ParitySummary,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DifficultyCharacteristic {
    Standard,
    OneSaber,
    NoArrows,
    #[serde(rename(serialize = "90Degree", deserialize = "90Degree"))]
    NinetyDegree,
    #[serde(rename(serialize = "360Degree", deserialize = "360Degree"))]
    ThreeSixtyDegree,
    Lightshow,
    Lawless,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DifficultyType {
    Easy,
    Normal,
    Hard,
    Expert,
    ExpertPlus,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ParitySummary {
    pub errors: u32,
    pub warns: u32,
    pub resets: u32,
}