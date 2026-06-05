use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{ Path, PathBuf };

use serde::{ Deserialize, Serialize };
use serde_json;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::cli::{ Mode, Service };

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
    Image,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Audio => "audio",
            MediaType::Video => "video",
            MediaType::Image => "image",
        }
    }
}

pub fn scan_directories(dir: &String) -> Result<HashMap<String, PathBuf>, Box<dyn Error>> {
    let mut file_map: HashMap<String, PathBuf> = HashMap::new();
    println!("[INFO] Starting scan directory: {:?}", dir);

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(os_str) = path.file_name() {
                let name = os_str.to_string_lossy().into_owned();
                file_map.insert(name, path);
            }
        }
    }

    Ok(file_map)
}

pub async fn save_file(
    file_name: String,
    target_dir: &str,
    bytes: bytes::Bytes
) -> Result<(), Box<dyn Error>> {
    let mut path = PathBuf::from(target_dir);
    path.push(file_name);

    let mut file = File::create(&path).await?;
    file.write_all(&bytes).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub dialog_mode: bool,
    pub service: Service,
    pub media_type: MediaType,
    pub audio_dir: String,
    pub video_dir: String,
    pub image_dir: String,
    pub mode: Mode,
    pub concurrency: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dialog_mode: true,
            service: Service::Yandex,
            media_type: MediaType::Audio,
            audio_dir: "./static/audio".to_string(),
            video_dir: "./static/video".to_string(),
            image_dir: "./static/image".to_string(),
            mode: Mode::All,
            concurrency: 5,
        }
    }
}

pub const CONFIG_PATH: &str = "./config.json";

pub fn read_config() -> Result<AppConfig, Box<dyn Error>> {
    let config: AppConfig = serde_json::from_str(&fs::read_to_string(&CONFIG_PATH)?)?;
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &AppConfig) -> Result<(), Box<dyn Error>> {
    let AppConfig { media_type, audio_dir, video_dir, image_dir, concurrency, .. } = config;

    if !(1..=10).contains(*&concurrency) {
        return Err(format!("Concurrency must be between 1 and 10 (was {})", concurrency).into());
    }

    if *media_type == MediaType::Audio && !Path::new(&audio_dir).is_dir() {
        return Err(
            format!("Audio directory does not exist or is not a folder: {}", audio_dir).into()
        );
    }
    if *media_type == MediaType::Video && !Path::new(&video_dir).is_dir() {
        return Err(
            format!("Video directory does not exist or is not a folder: {}", video_dir).into()
        );
    }
    if *media_type == MediaType::Image && !Path::new(&image_dir).is_dir() {
        return Err(
            format!("Image directory does not exist or is not a folder: {}", image_dir).into()
        );
    }

    Ok(())
}

pub fn write_config() -> Result<(), Box<dyn Error>> {
    let default_config = AppConfig::default();
    let json_string = serde_json::to_string_pretty(&default_config)?;
    fs::write(&CONFIG_PATH, json_string)?;
    Ok(())
}
