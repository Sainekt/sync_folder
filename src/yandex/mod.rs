use crate::get_reqwest_client;
use crate::storage::{ save_file, DirNames };
use futures::{ stream, StreamExt };
use reqwest::header::{ HeaderValue, AUTHORIZATION };
use reqwest::Method;
use reqwest::{ StatusCode, Url };
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

// =====================================================================================================================

const LIMIT: &str = "100000";
const YANDEX_URL: &str = "https://cloud-api.yandex.net/v1/disk/resources";

// =====================================================================================================================
pub struct YandexService {
    token: String,
    concurrency: usize,
    media_type: DirNames,
}

impl YandexService {
    pub fn new(token: String, concurrency: usize, media_type: DirNames) -> Self {
        Self { token, concurrency, media_type }
    }
    pub async fn fetch_metadata(
        &self
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn Error>> {
        let mut url = Url::parse(YANDEX_URL)?;
        url.query_pairs_mut()
            .append_pair("limit", LIMIT)
            .append_pair("path", &format!("app:/{}", self.media_type.as_str()));

        let (status, response) = Self::fetch(&self, url, Method::GET, None).await?;

        if status == reqwest::StatusCode::OK {
            return Ok(response);
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            match Self::create_cloud_dir(&self, self.media_type.clone()).await {
                Ok(_) => {
                    println!("[INFO] Directory successfully created");
                    let fake_json =
                        serde_json::json!({
                    "_embedded": {
                        "items": []
                    }
                });

                    return Ok(fake_json.as_object().unwrap().clone().into_iter().collect());
                }
                Err(e) => {
                    return Err(format!("[ERROR] failed create directory: {}", e).into());
                }
            }
        }

        let err_msg = response
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error from Yandex_api");

        Err(format!("Yandex_api returned error: {}", err_msg).into())
    }

    pub fn parse_response(
        &self,
        response: HashMap<String, serde_json::Value>
    ) -> Result<ParsedResult, Box<dyn Error>> {
        let mut result: ParsedResult = ParsedResult {
            audio: Vec::new(),
            image: Vec::new(),
            video: Vec::new(),
        };

        if
            let Some(items_array) = response
                .get("_embedded")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("items"))
                .and_then(|v| v.as_array())
        {
            for item in items_array {
                if let Some(file_obj) = item.as_object() {
                    let name: String = file_obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let item_type: &str = file_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let is_dir: bool = item_type == "dir";

                    let size: u64 = file_obj
                        .get("size")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    let path: String = file_obj
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let media_type: String = file_obj
                        .get("media_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if !name.is_empty() {
                        if media_type == "audio" {
                            result.audio.push(CloudItem {
                                name,
                                is_dir,
                                size,
                                path,
                                media_type,
                            });
                        } else if media_type == "image" {
                            result.image.push(CloudItem {
                                name,
                                is_dir,
                                size,
                                path,
                                media_type,
                            });
                        } else if media_type == "video" {
                            result.video.push(CloudItem {
                                name,
                                is_dir,
                                size,
                                path,
                                media_type,
                            });
                        }
                    }
                }
            }
        } else {
            return Err("Failed to parse Yandex API response: missing_embedded.items".into());
        }

        Ok(result)
    }

    pub async fn download(&self, paths: Vec<CloudItem>) -> Result<(), Box<dyn Error>> {
        let client = Arc::new(get_reqwest_client());
        let base_url = Url::parse("https://cloud-api.yandex.net/v1/disk/resources/download")?;

        let download_stream = stream
            ::iter(paths)
            .map(|cloud_item| {
                let client = Arc::clone(&client);
                let mut get_download_url = base_url.clone();

                get_download_url.query_pairs_mut().append_pair("path", cloud_item.path.as_str());

                async move {
                    let (status, response) = Self::fetch(
                        &self,
                        get_download_url,
                        Method::GET,
                        Some(&client)
                    ).await.map_err(|e| format!("[ERROR] failed to fetch download link: {}", e))?;

                    if status != StatusCode::OK {
                        let error_msg = response
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        return Err(
                            format!(
                                "[ERROR] {} | Failed to get download URL: {}",
                                cloud_item.name,
                                error_msg
                            )
                        );
                    }

                    let Some(download_url) = response.get("href").and_then(|v| v.as_str()) else {
                        return Err(
                            format!("[ERROR] {} | Missing href in response", cloud_item.name)
                        );
                    };

                    println!("[DOWNLOAD] Starting download: {}", cloud_item.name);
                    let start_time = Instant::now();

                    let file_response = match client.get(download_url).send().await {
                        Ok(res) => {
                            if !res.status().is_success() {
                                return Err(
                                    format!(
                                        "[ERROR] {} | Server returned error status: {}",
                                        cloud_item.name,
                                        res.status()
                                    )
                                );
                            }
                            res
                        }
                        Err(e) => {
                            return Err(
                                format!("[ERROR] download failed: {}, {}", cloud_item.name, e)
                            );
                        }
                    };

                    let bytes = match file_response.bytes().await {
                        Ok(b) => b,
                        Err(_) => {
                            return Err(
                                format!("[ERROR] failed read bytes from file: {}", cloud_item.name)
                            );
                        }
                    };

                    //Stats
                    // =========================================================================================================

                    let duration = start_time.elapsed();
                    let bytes_len = bytes.len();
                    let megabytes = (bytes_len as f64) / 1_048_576.0;
                    let seconds = duration.as_secs_f64();
                    let speed = if seconds > 0.0 { megabytes / seconds } else { 0.0 };

                    // =========================================================================================================
                    let file_name = cloud_item.name.clone();

                    println!(
                        "[STATS] Downloaded {:.2} MB in {:.2}s | Speed: {:.2} MB/s | File: {}",
                        megabytes,
                        seconds,
                        speed,
                        file_name
                    );

                    match save_file(cloud_item, bytes).await {
                        Ok(()) => {
                            println!("[SUCCESS] file is saved successfully: {}", file_name);
                        }
                        Err(e) => {
                            return Err(format!("[ERROR] failed save file: {}", e));
                        }
                    }
                    Ok(())
                }
            })
            .buffer_unordered(self.concurrency);

        let result: Vec<Result<(), String>> = download_stream.collect().await;

        let mut errors_count = 0;
        for res in result {
            if let Err(err_msg) = res {
                println!("{}", err_msg);
                errors_count += 1;
            }
        }

        if errors_count > 0 {
            println!("[INFO] Batch download finished with {} errors.", errors_count);
        } else {
            println!("[INFO] All files download successfully!");
        }

        Ok(())
    }

    pub async fn upload(
        &self,
        to_upload: HashMap<String, PathBuf>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = Arc::new(get_reqwest_client());
        let dir = Arc::new(self.media_type.clone());

        let upload_stream = stream
            ::iter(to_upload)
            .map(|(file_name, file_path)| {
                let client = Arc::clone(&client);
                let dir = Arc::clone(&dir);

                async move {
                    let mut url = Url::parse(&format!("{}/upload", YANDEX_URL)).map_err(|e|
                        format!("[ERROR] url parse failed: {}", e)
                    )?;

                    url.query_pairs_mut().append_pair(
                        "path",
                        &format!("app:/{}/{}", dir.as_str(), file_name)
                    );

                    let (status, response) = Self::fetch(
                        &self,
                        url,
                        Method::GET,
                        Some(&client)
                    ).await.map_err(|e| format!("[ERROR] Fetch failed: {}", e))?;

                    if status != StatusCode::OK {
                        let error_msg = response
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        return Err(
                            format!("[ERROR] {} | Failed to get URL: {}", file_name, error_msg)
                        );
                    }

                    let Some(upload_url) = response.get("href").and_then(|v| v.as_str()) else {
                        return Err(format!("[ERROR] {} | Missing href in response", file_name));
                    };

                    println!("[UPLOAD] Starting upload: {}", file_name);
                    let start_time = std::time::Instant::now();

                    let file_bytes = std::fs
                        ::read(&file_path)
                        .map_err(|e| format!("[ERROR] {} | Read failed: {}", file_name, e))?;
                    let bytes_len = file_bytes.len();

                    let res = client
                        .put(upload_url)
                        .body(file_bytes)
                        .send().await
                        .map_err(|e| format!("[ERROR] {} | Network error: {}", file_name, e))?;

                    if res.status() == StatusCode::CREATED || res.status() == StatusCode::ACCEPTED {
                        let duration = start_time.elapsed();
                        let megabytes = (bytes_len as f64) / 1_048_576.0;
                        let seconds = duration.as_secs_f64();
                        let speed = if seconds > 0.0 { megabytes / seconds } else { 0.0 };

                        println!(
                            "[SUCCESS] Uploaded {:.2} MB in {:.2}s | Speed: {:.2} MB/s | File: {}",
                            megabytes,
                            seconds,
                            speed,
                            file_name
                        );
                        Ok(())
                    } else {
                        Err(
                            format!(
                                "[ERROR] {} | Yandex rejected with status: {}",
                                file_name,
                                res.status()
                            )
                        )
                    }
                }
            })
            .buffer_unordered(self.concurrency);

        let results: Vec<Result<(), String>> = upload_stream.collect().await;

        let mut errors_count = 0;
        for res in results {
            if let Err(err_msg) = res {
                println!("{}", err_msg);
                errors_count += 1;
            }
        }

        if errors_count > 0 {
            println!("[INFO] Batch upload finished with {} errors.", errors_count);
        } else {
            println!("[INFO] All files uploaded successfully!");
        }

        Ok(())
    }

    async fn fetch(
        &self,
        url: Url,
        method: reqwest::Method,
        client: Option<&reqwest::Client>
    ) -> Result<(StatusCode, HashMap<String, Value>), Box<dyn std::error::Error>> {
        let auth_value = format!("OAuth {}", self.token);

        let client = client.unwrap_or_else(|| get_reqwest_client());

        let resp = client
            .request(method, url)
            .header(AUTHORIZATION, HeaderValue::from_str(&auth_value)?)
            .send().await?;

        let status = resp.status();

        let body = resp.json::<HashMap<String, Value>>().await?;

        Ok((status, body))
    }

    async fn create_cloud_dir(&self, dir: DirNames) -> Result<StatusCode, Box<dyn Error>> {
        let mut url = Url::parse(YANDEX_URL)?;
        url.query_pairs_mut()
            .append_pair("limit", LIMIT)
            .append_pair("path", &format!("app:/{}", dir.as_str()));

        let status = Self::fetch(&self, url, Method::PUT, None).await?.0;
        if status != reqwest::StatusCode::CREATED {
            return Err(format!("Failed to create directory status: {}", status).into());
        }
        Ok(status)
    }
}
// Structures
// =====================================================================================================================
#[derive(Debug)]
pub struct CloudItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub path: String,
    pub media_type: String,
}

#[derive(Debug, Default)]
pub struct ParsedResult {
    pub audio: Vec<CloudItem>,
    pub image: Vec<CloudItem>,
    pub video: Vec<CloudItem>,
}
