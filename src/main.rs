mod yandex;
mod storage;
mod cli;
use std::{ collections::HashMap, error::Error, path::PathBuf, sync::OnceLock };

use yandex::YandexService;
use storage::scan_directories;
use dotenv::dotenv;

use crate::{ cli::{ Choices, Service, choice_mode, parse_args }, yandex::CloudItem };

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let choices = parse_args()?;
    if choices.service == Service::Yandex {
        with_yandex_drive(choices).await?;
    }

    Ok(())
}

// Yandex
// =====================================================================================================================
async fn with_yandex_drive(choices: Choices) -> Result<(), Box<dyn Error>> {
    let service = YandexService::new(
        choices.token,
        choices.concurrency,
        choices.media_type.clone()
    );

    let files_info = service.fetch_metadata().await?;
    let parsed_info = service.parse_response(files_info)?;

    let data_in_dir = scan_directories(choices.media_type.clone())?;
    let to_sync_media_type = match choices.media_type {
        storage::DirNames::Audio => parsed_info.audio,
        storage::DirNames::Image => parsed_info.image,
        storage::DirNames::Video => parsed_info.video,
    };

    let to_sync = calculate_sync_tasks(to_sync_media_type, data_in_dir);
    let mode = choice_mode()?;
    match mode {
        cli::Mode::All => {
            service.download(to_sync.to_download).await?;
            service.upload(to_sync.to_upload).await?;
        }
        cli::Mode::Download => service.download(to_sync.to_download).await?,
        cli::Mode::Upload => service.upload(to_sync.to_upload).await?,
    }

    Ok(())
}

// General
// =====================================================================================================================
#[derive(Debug)]
pub struct SyncTasks {
    pub to_download: Vec<CloudItem>,
    pub to_upload: HashMap<String, PathBuf>,
}

fn calculate_sync_tasks(
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

    &CLIENT.get_or_init(|| { reqwest::Client::new() })
}
