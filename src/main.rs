mod yandex;
mod storage;
mod cli;
use std::{ collections::HashMap, error::Error, path::{ Path, PathBuf }, sync::OnceLock };

use yandex::YandexService;
use storage::scan_directories;
use dotenv::dotenv;

use crate::{
    cli::{
        Choices,
        Mode,
        Service,
        WorkMode,
        YesOrNot,
        choice_mode,
        choice_work_mode,
        input_token,
        parse_args,
        yes_or_not,
    },
    storage::{ AppConfig, CONFIG_PATH, MediaType, read_config, write_config },
    yandex::CloudItem,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    if Path::new(CONFIG_PATH).exists() {
        return with_config().await;
    }
    let work_mode = choice_work_mode()?;

    if work_mode == WorkMode::Config {
        println!("[INFO] Config is not created, start to create...");
        write_config()?;
        println!("[SUCCESS] Config success created, see config.json");
        println!(
            "[INFO] You can continue in dialog mode, or modify the config and restart the program."
        );

        let proceed = yes_or_not("Continue with dialog mode ?:")?;
        if proceed == YesOrNot::No {
            return Ok(());
        }
    }

    with_dialog().await
}

// with_config
// =====================================================================================================================
async fn with_config() -> Result<(), Box<dyn Error>> {
    let config = match read_config() {
        Ok(c) => c,
        Err(e) => {
            return Err(
                format!("[ERROR] The 'config.json' file is invalid: {}. Please fix the configuration, or delete the file to return to dialog mode.", e).into()
            );
        }
    };
    println!("[INFO] Using configuration from 'config.json'");

    if config.dialog_mode {
        return with_dialog().await;
    }

    let AppConfig { service, media_type, audio_dir, video_dir, image_dir, mode, concurrency, .. } =
        config;

    let target_dir = match media_type {
        MediaType::Audio => audio_dir,
        MediaType::Video => video_dir,
        MediaType::Image => image_dir,
    };

    // =========================================================================

    let env_key = if service == Service::Yandex { "YANDEX_TOKEN" } else { "GOOGLE_TOKEN" };
    let token = std::env::var(env_key).or_else(|_| {
        println!("[INFO]{} is not set in the .env file.", env_key);
        input_token(Some("Please enter your token manually:"))
    })?;

    if token.trim().is_empty() {
        return Err(format!("[ERROR] Token for {} cannot be empty.", env_key).into());
    }

    // =========================================================================

    let choices = Choices {
        concurrency,
        media_type,
        target_dir,
        service: service.clone(),
        token,
    };

    if service == Service::Yandex {
        return with_yandex_drive(choices, Some(mode)).await;
    }

    Ok(())
}

// with_dialog
// =====================================================================================================================
async fn with_dialog() -> Result<(), Box<dyn Error>> {
    let choices = parse_args()?;
    if choices.service == Service::Yandex {
        with_yandex_drive(choices, None).await?;
    }

    Ok(())
}

// Yandex
// =====================================================================================================================
async fn with_yandex_drive(choices: Choices, mode: Option<Mode>) -> Result<(), Box<dyn Error>> {
    let service = YandexService::new(
        choices.token,
        choices.concurrency,
        choices.media_type.clone()
    );

    let files_info = service.fetch_metadata().await?;
    let parsed_info = service.parse_response(files_info)?;

    let data_in_dir = scan_directories(&choices.target_dir)?;
    let to_sync_media_type = match choices.media_type {
        storage::MediaType::Audio => parsed_info.audio,
        storage::MediaType::Image => parsed_info.image,
        storage::MediaType::Video => parsed_info.video,
    };

    let to_sync = calculate_sync_tasks(to_sync_media_type, data_in_dir);
    let mode = match mode {
        Some(m) => m,
        None => choice_mode()?,
    };

    match mode {
        cli::Mode::All => {
            service.download(to_sync.to_download, &choices.target_dir).await?;
            service.upload(to_sync.to_upload).await?;
        }
        cli::Mode::Download => service.download(to_sync.to_download, &choices.target_dir).await?,
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
