mod yandex;
mod storage;

use std::{ collections::{ HashMap }, path::PathBuf, sync::OnceLock };

use yandex::{ fetch_metadata, parse_response, download };
use storage::scan_directories;
use dotenv::dotenv;

use crate::{ yandex::{ CloudItem, upload } };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let media_type = storage::DirNames::Audio;

    let meta_data = fetch_metadata(media_type.clone()).await?;
    let parsed_yandex_response = parse_response(meta_data)?;

    let data_in_dir = scan_directories(media_type.clone())
        .inspect_err(|e| println!("[ERROR] {}", e))
        .unwrap_or_default();

    let to_sync_media_type = match media_type {
        storage::DirNames::Audio => parsed_yandex_response.audio,
        storage::DirNames::Image => parsed_yandex_response.image,
        storage::DirNames::Video => parsed_yandex_response.video,
    };

    let to_sync = get_to_download(to_sync_media_type, data_in_dir);

    download(to_sync.to_download).await?;
    upload(media_type, to_sync.to_upload).await?;

    Ok(())
}

#[derive(Debug)]
pub struct SyncTasks {
    pub to_download: Vec<CloudItem>,
    pub to_upload: HashMap<String, PathBuf>,
}

fn get_to_download(
    from_drive: Vec<CloudItem>,
    mut data_in_dir: HashMap<String, PathBuf>
) -> SyncTasks {
    let mut to_download = Vec::new();

    for entry in from_drive {
        if data_in_dir.remove(&entry.name).is_some() {
            continue;
        }
        to_download.push(entry);
    }

    let to_upload = data_in_dir;

    println!(
        "[INFO] To download {} files. To upload {} files.",
        to_download.len(),
        to_upload.len()
    );

    SyncTasks { to_download, to_upload }
}

pub fn get_reqwest_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    &CLIENT.get_or_init(|| {
        println!("[INFO] Init reqwest Client.");
        reqwest::Client::new()
    })
}
