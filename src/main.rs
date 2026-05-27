mod yandex;
mod storage;

use std::{collections::HashSet, sync::OnceLock};

use yandex::execute_with_yandex;
use storage::scan_directories;
use dotenv::dotenv;

use crate::yandex::{CloudItem, download};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let media_type = storage::DirNames::Audio;

    let data = execute_with_yandex().await
        .inspect_err(|e| println!("[ERROR] error execute with yandex: {}", e))
        .unwrap_or_default();


    let data_in_dir = scan_directories(media_type.clone())
        .inspect_err(|e| println!("[ERROR] {}", e))
        .unwrap_or_default();

    let to_sync_media_type = match media_type {
        storage::DirNames::Audio => data.audio,
        storage::DirNames::Image => data.image,
        storage::DirNames::Video => data.video,
    };
    
    let to_sync = get_to_download(to_sync_media_type, data_in_dir);
    
    download(to_sync.to_download).await?;

    Ok(())
}



#[derive(Debug)]
pub struct SyncTasks {
    pub to_download: Vec<CloudItem>,
    pub to_upload: Vec<String>
}

fn get_to_download(from_drive: Vec<CloudItem>, data_in_dir: HashSet<String>) -> SyncTasks {
    let mut already_has = data_in_dir;
    let mut to_download:Vec<CloudItem> =  Vec::new();
    for entry in from_drive {
        if already_has.remove(&entry.name) {
                    continue;
                }
        to_download.push(entry);
    }

    println!("[INFO] To download {} files. To upload {} files.", to_download.len(), already_has.len());
    SyncTasks { to_download: to_download, to_upload: already_has.into_iter().collect() }
}

pub fn get_reqwest_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    &CLIENT.get_or_init(|| {
        println!("[INFO] Init reqwest Client.");
        reqwest::Client::new()
    })
}