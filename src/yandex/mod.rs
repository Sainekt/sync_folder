use std::path::PathBuf;
use std::time::Instant;
use std::error::Error;
use std::collections::{HashMap};
use std::{env};
use reqwest::{ StatusCode, Url};
use reqwest::header::{AUTHORIZATION, HeaderValue};
use reqwest::Method;
use serde_json::Value;
use crate::get_reqwest_client;
use crate::storage::{ DirNames, save_file};

// =====================================================================================================================

const LIMIT: &str = "100000";
const YANDEX_URL: &str = "https://cloud-api.yandex.net/v1/disk/resources";

// =====================================================================================================================

pub async fn fetch_metadata(dir: DirNames) -> Result<HashMap<String, serde_json::Value>, Box<dyn Error>>  {
    let mut url = Url::parse(YANDEX_URL)?;
        url.query_pairs_mut()
        .append_pair("limit", LIMIT)
        .append_pair("path", &format!("app:/{}", dir.as_str()));
    
    let (status, response)  = fetch(url,Method::GET, None).await?;

    if status == reqwest::StatusCode::OK {
            return Ok(response);
        }

    
    if status == reqwest::StatusCode::NOT_FOUND {
        match create_cloud_dir(dir).await {
                    Ok(_) => {
                        println!("[INFO] Directory successfully created");
                        let fake_json = serde_json::json!({
                            "_embedded": {
                                "items": []
                            }
                        });

                        return Ok(fake_json.as_object().unwrap().clone().into_iter().collect());
                    },
                    Err(e) => {
                        return Err(format!("[ERROR] failed create directory: {}", e).into());
                    }
                }
    } 

    let err_msg = response.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error from Yandex_api");
    
    Err(format!("Yandex_api returned error: {}", err_msg).into())
}

pub fn parse_response(response: HashMap<String, serde_json::Value>) -> Result<ParsedResult, Box<dyn Error>> {
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
                        result.audio.push(CloudItem { name, is_dir, size, path, media_type });
                    } 
                    else if media_type == "image" {
                        result.image.push(CloudItem { name, is_dir, size, path, media_type });
                    }
                    else if media_type =="video" {
                        result.video.push(CloudItem { name, is_dir, size, path, media_type });
                    }
                }
            }
        }
    } else {
        return Err("Failed to parse Yandex API response: missing_embedded.items".into());
    }
    
    Ok(result)
}


pub async fn download(paths: Vec<CloudItem>) -> Result<(), Box<dyn Error>> {
    let client = get_reqwest_client();
    let base_url = Url::parse("https://cloud-api.yandex.net/v1/disk/resources/download")?;

    for cloud_item in paths {
        let url = base_url.clone();
        let params = [("path", cloud_item.path.as_str())];
        let response = fetch(url,Method::GET,  Some(&params)).await?.1;

        if let Some(download_url) = response.get("href").and_then(|v| v.as_str()) {
            println!("[DOWNLOAD] starting download: {:?}", cloud_item.name);
            let start_time = Instant::now();

            let file_response = match client.get(download_url).send().await {
                Ok(res) => res,
                Err(e) => {
                    println!("[ERROR] download failed: {}, {}", cloud_item.name, e);
                    continue;
                }
            };

            let bytes = match file_response.bytes().await {
                Ok(b) => b,
                Err(_) => {
                    println!("[ERROR] failed read bytes from file: {}", cloud_item.name);
                    continue;
                }
            };

            //Stats
            // =========================================================================================================

            let duration = start_time.elapsed();
            let bytes_len = bytes.len();
            let megabytes = bytes_len as f64 / 1_048_576.0;
            let seconds = duration.as_secs_f64();
            let speed = if seconds > 0.0 { megabytes / seconds } else { 0.0 };

            // =========================================================================================================

            println!(
                "[STATS] Downloaded {:.2} MB in {:.2}s | Speed: {:.2} MB/s", 
                megabytes, seconds, speed
            );

            match save_file(cloud_item, bytes).await {
                Ok(()) => {
                    println!("[SUCCESS] file is saved successfully")
                },
                Err(e) => {
                    println!("[ERROR] failed save file: {}", e);
                    continue;
                }
            };

            

        } 
    }

    Ok(())
}

pub async fn upload(dir: DirNames, to_upload: HashMap<String, PathBuf>) -> Result<(), Box<dyn Error>> {
    let client = get_reqwest_client();

    for (file_name, file_path) in to_upload {
        let mut url = Url::parse(&format!("{}/upload", YANDEX_URL))?;
        url.query_pairs_mut()
            .append_pair("path", &format!("app:/{}/{}", dir.as_str(), file_name));

        let (status, response) = fetch(url, Method::GET, None).await?;

        if status != reqwest::StatusCode::OK {
            let err_msg = response.get("message")
                .and_then(|v| v.as_str()).unwrap_or("No message");
            return Err(format!("[ERROR] Upload link is not get, Status: {} message: {:?}",status, err_msg).into());
        }

        let Some(upload_url) = response.get("href").and_then(|v| v.as_str()) else {
            return  Err("[ERROR] Yandex response is missing 'href' field to upload".into());
        };

        println!("[UPLOAD] Preparing to upload: {}", file_name);
                let start_time = Instant::now();

        let file_bytes = std::fs::read(&file_path).map_err(|e| {
            format!("Failed to read local file {:?}: {}", file_path, e)
        })?;

        let bytes_len = file_bytes.len();

        let response = client
            .put(upload_url)
            .body(file_bytes)
            .send()
            .await
            .map_err(|e| format!("Network error during upload for {}: {}", file_name, e))?;

        if response.status() == StatusCode::CREATED || response.status() == StatusCode::ACCEPTED {
            let duration = start_time.elapsed();
            let megabytes = bytes_len as f64 / 1_048_576.0;
            let seconds = duration.as_secs_f64();
            let speed = if seconds > 0.0 { megabytes / seconds } else { 0.0 };

            println!(
                "[SUCCESS] Uploaded {:.2} MB in {:.2}s | Speed: {:.2} MB/s | File: {}", 
                megabytes, seconds, speed, file_name
            );
        } else {
            return Err(format!("Yandex rejected file {} with status: {}", file_name, response.status()).into());
        }
    }
    Ok(())
}

async fn fetch(mut url: Url, method: reqwest::Method, query: Option<&[(&str, &str)]>)
 -> Result<(StatusCode, HashMap<String, Value>), Box<dyn std::error::Error>> {
    let yandex_token = env::var("YANDEX_TOKEN")
        .map_err(|_| "YANDEX_TOKEN is not set into .env file")?;

    if let Some(params) = query {
            let mut pairs = url.query_pairs_mut();
            for &(key, val) in params {
                pairs.append_pair(key, val);
            }
        }

    let auth_value = format!("OAuth {}", yandex_token);

    let client = get_reqwest_client();
    
    let resp = client
        .request(method, url)
        .header(AUTHORIZATION, HeaderValue::from_str(&auth_value)?)
        .send()
        .await?;

    let status = resp.status();

    let body = resp.
        json::<HashMap<String, Value>>()
        .await?;

    Ok((status, body))
}

async fn create_cloud_dir(dir: DirNames) -> Result<StatusCode, Box<dyn Error>>{
    let mut url = Url::parse(YANDEX_URL)?;
        url.query_pairs_mut()
        .append_pair("limit", LIMIT)
        .append_pair("path", &format!("app:/{}", dir.as_str()));

    let status = fetch(url, Method::PUT, None).await?.0;
    if status != reqwest::StatusCode::CREATED {
        return Err(format!("Failed to create directory status: {}", status).into());
    }
    Ok(status)
}

// Structures
// =====================================================================================================================
#[derive(Debug)]
pub struct CloudItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub path: String,
    pub media_type: String
}

#[derive(Debug,Default)]
pub struct ParsedResult {
    pub audio: Vec<CloudItem>,
    pub image: Vec<CloudItem>,
    pub video: Vec<CloudItem>
}