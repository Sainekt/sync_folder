use reqwest::header::{AUTHORIZATION, HeaderValue};
use std::collections::HashMap;
use std::{env};

use std::error::Error;
const LIMIT: &str = "100000";
const YANDEX_APP_URL: &str = "https://cloud-api.yandex.net/v1/disk/resources";

async fn fetch_metadata() -> Result<HashMap<String, serde_json::Value>, Box<dyn Error>>  {
    let yandex_token = match env::var("YANDEX_TOKEN") {
        Ok(token) => token,
        Err(_) => return  Err("YANDEX_TOKEN is not set into .env file".into())
    };

    let auth_value = format!("OAuth {}", yandex_token);
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}?limit={}&path=app:/",YANDEX_APP_URL, LIMIT))
        .header(AUTHORIZATION, HeaderValue::from_str(&auth_value)?)
        .send()
        .await?
        .json::<HashMap<String, serde_json::Value>>()
        .await?;

    if resp.contains_key("error") {
        let err_msg = resp.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error from Yandex_api");

        return Err(format!("Yandex_api returned error: {}", err_msg).into());
    }

    Ok(resp)
}


#[derive(Debug)]
pub struct CloudItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub path: String
}

#[derive(Debug)]
struct ParsedResult {
    pub audio: Vec<CloudItem>,
    pub image: Vec<CloudItem>,
    pub video: Vec<CloudItem>
}

fn parse_response(response: HashMap<String, serde_json::Value>) -> Result<ParsedResult, Box<dyn Error>> {
    let mut result: ParsedResult = ParsedResult {
        audio: Vec::new(),
        image: Vec::new(),
        video: Vec::new(),
    };

    if let Some(items_array) = response.get("_embedded")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("items"))
        .and_then(|v| v.as_array()) 
    {
        for item in items_array {
            if let Some(file_obj) = item.as_object() {
                
                let name: String = file_obj.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let item_type: &str= file_obj.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let is_dir: bool = item_type == "dir";

                let size: u64 = file_obj.get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let path: String = file_obj.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_string();

                let media_type: String = file_obj.get("media_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_string();

                if !name.is_empty() {
                    if media_type == "audio" {
                        result.audio.push(CloudItem { name, is_dir, size, path });
                    } 
                    else if media_type == "image" {
                        result.image.push(CloudItem { name, is_dir, size, path });
                    }
                    else if media_type =="video" {
                        result.video.push(CloudItem { name, is_dir, size, path });
                    }
                }
            }
        }
    } else {
        return Err("Failed to parse Yandex API response: missing_embedded.items".into());
    }
    
    Ok(result)
}


pub async fn execute_with_yandex() -> Result<(), Box<dyn std::error::Error>> {
    let resp = fetch_metadata().await?;
    let parsed_response = parse_response(resp)?;
    println!("[mod]: {:#?}", parsed_response);
    Ok(())
}