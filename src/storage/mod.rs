
use std::collections::{HashMap, HashSet};
use std::fs;
use std::error::Error;
use std::path::PathBuf;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::yandex::CloudItem;

#[derive(Debug, Clone)]
pub enum DirNames {
    Audio,
    Video,
    Image,
}
impl DirNames {
    pub fn as_str(&self) -> &'static str {
        match self {
            DirNames::Audio => "audio",
            DirNames::Video => "video",
            DirNames::Image => "image",
            
        }
    }
}


pub fn scan_directories(dir: DirNames) -> Result<HashMap<String, PathBuf>, Box<dyn Error>> {
    let mut file_map: HashMap<String, PathBuf> = HashMap::new();

    let folder = dir.as_str();
    let target_path = format!("./static/{}", folder);

    fs::create_dir_all(&target_path).map_err(|e| {
        format!("Failed to create directory '{}': {}", target_path, e)
    })?;

    println!("[INFO] Starting scan directory: {:?}", dir);
    
    for entry in fs::read_dir(&target_path)? {
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


pub async fn save_file(file_info: CloudItem, bytes: bytes::Bytes ) -> Result<(), Box<dyn Error>> {
    let mut path = PathBuf::from("./static");
        path.push(&file_info.media_type);
        path.push(&file_info.name);

        let mut file = File::create(&path).await?;
        file.write_all(&bytes).await?;
        Ok(())
}