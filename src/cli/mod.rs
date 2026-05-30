use inquire::{ CustomType, InquireError, Select, Text, validator::Validation };
use std::{ error::Error, fmt::{ self, Display }, vec };

use crate::storage::DirNames;
use std::env;

#[derive(Debug, Clone)]
pub struct Choices {
    pub token: String,
    pub service: Service,
    pub media_type: DirNames,
    pub concurrency: usize,
}

pub fn parse_args() -> Result<Choices, Box<dyn Error>> {
    let service = choice_service()?;
    if service == Service::Google {
        return Err(format!("Google support is not implemented yet.").into());
    }

    let token = match input_token() {
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
    let concurrency = choice_concurrency()?;

    return Ok(Choices { token, service, media_type, concurrency });
}

// =====================================================================================================================
#[derive(Debug, Clone, PartialEq)]
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
impl fmt::Display for DirNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Audio => write!(f, "🎵 Audio"),
            Self::Video => write!(f, "🎬 Video"),
            Self::Image => write!(f, "📸 Image"),
        }
    }
}
fn choice_media_type() -> Result<DirNames, InquireError> {
    let option = vec![DirNames::Audio, DirNames::Video, DirNames::Image];
    let media_type = Select::new("Choice media type:", option).prompt()?;
    Ok(media_type)
}

// =====================================================================================================================
fn input_token() -> Result<String, InquireError> {
    let token = Text::new("Input an app token:").prompt()?;
    Ok(token)
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
