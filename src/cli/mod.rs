use inquire::{ CustomType, InquireError, Select, Text, validator::Validation };
use serde::{ Deserialize, Serialize };
use std::{ error::Error, fmt::{ self, Display }, path::{ Path }, vec };

use crate::{ storage::MediaType };
use std::env;

#[derive(Debug, Clone)]
pub struct Choices {
    pub token: String,
    pub service: Service,
    pub media_type: MediaType,
    pub concurrency: usize,
    pub target_dir: String,
}

pub fn parse_args() -> Result<Choices, Box<dyn Error>> {
    let service = choice_service()?;
    if service == Service::Google {
        return Err(format!("Google support is not implemented yet.").into());
    }

    let token = match input_token(None) {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            println!("💡 No token entered. Using the token from the .env file.");
            if service == Service::Yandex {
                env::var("YANDEX_TOKEN").expect(
                    "YANDEX_TOKEN is not set into .env file or environment variables"
                )
            } else {
                env::var("GOOGLE_TOKEN").expect(
                    "GOOGLE_TOKEN is not set into .env file or environment variables"
                )
            }
        }
    };

    let media_type = choice_media_type()?;

    let target_dir = input_target_dir()?;

    if target_dir.trim().is_empty() {
        return Err("[ERROR] Target directory path cannot be empty.".into());
    }

    if !Path::new(&target_dir).is_dir() {
        return Err(
            format!("[ERROR] Target directory does not exist or is not a folder: '{}'", target_dir).into()
        );
    }

    let concurrency = choice_concurrency()?;

    return Ok(Choices { token, service, media_type, concurrency, target_dir });
}

// =====================================================================================================================
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq)]
pub enum Service {
    Yandex,
    Google,
}
impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Google => write!(f, "Google"),
            Self::Yandex => write!(f, "Yandex"),
        }
    }
}

fn choice_service() -> Result<Service, InquireError> {
    let services = vec![Service::Yandex];
    let service = Select::new("Choice drive:", services).prompt()?;
    Ok(service)
}

// =====================================================================================================================
impl Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Audio => write!(f, "🎵 Audio"),
            Self::Video => write!(f, "🎬 Video"),
            Self::Image => write!(f, "📸 Image"),
        }
    }
}
fn choice_media_type() -> Result<MediaType, InquireError> {
    let option = vec![MediaType::Audio, MediaType::Video, MediaType::Image];
    let media_type = Select::new("Choice media type:", option).prompt()?;
    Ok(media_type)
}

// =====================================================================================================================
pub fn input_token(msg: Option<&str>) -> Result<String, InquireError> {
    let token = Text::new(msg.unwrap_or("Input an app token:")).prompt()?;
    Ok(token)
}

fn input_target_dir() -> Result<String, InquireError> {
    let path = Text::new("Input target local dir path:").prompt()?;
    return Ok(path);
}

// =====================================================================================================================
fn choice_concurrency() -> Result<usize, InquireError> {
    let threads = CustomType::<usize>
        ::new("Select maximum concurrency:")
        .with_default(5)
        .with_validator(|&input: &usize| {
            if (1..=10).contains(&input) {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid("Please enter a number between 1 and 10.".into()))
            }
        })
        .with_help_message("Enter a number between 1 and 10.")
        .prompt()?;

    Ok(threads)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Mode {
    Download,
    Upload,
    All,
}
impl Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Download => write!(f, "download"),
            Self::Upload => write!(f, "upload"),
        }
    }
}

pub fn choice_mode() -> Result<Mode, InquireError> {
    let modes = vec![Mode::All, Mode::Download, Mode::Upload];
    let mode = Select::new("Choice mode:", modes).prompt()?;
    Ok(mode)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkMode {
    Config,
    Dialog,
}
impl Display for WorkMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config => write!(f, "with config"),
            Self::Dialog => write!(f, "dialog"),
        }
    }
}

pub fn choice_work_mode() -> Result<WorkMode, InquireError> {
    let modes = vec![WorkMode::Dialog, WorkMode::Config];
    let mode = Select::new("Proceed with dialog or create config?:", modes).prompt()?;
    Ok(mode)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum YesOrNot {
    Yes,
    No,
}
impl Display for YesOrNot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::No => write!(f, "No"),
            Self::Yes => write!(f, "Yes"),
        }
    }
}

pub fn yes_or_not(msg: &str) -> Result<YesOrNot, InquireError> {
    let variant = vec![YesOrNot::Yes, YesOrNot::No];
    let answer = Select::new(msg, variant).prompt()?;
    Ok(answer)
}
