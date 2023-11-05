use std::{collections::HashMap, time::SystemTime};

use rand::{Rng, SeedableRng};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Response, StatusCode,
};
use serde::{Deserialize, Serialize};

const DEEPL_API: &str = "https://www2.deepl.com/jsonrpc";

#[derive(Serialize, Debug)]
pub struct Lang<'a> {
    pub source_lang_user_selected: &'a str,
    pub target_lang: &'a str,
}

#[derive(Serialize, Debug)]
pub struct CommonJobParams<'a> {
    pub was_spoken: bool,
    pub transcribe_as: &'a str,
}

#[derive(Serialize, Debug)]
pub struct Params<'a> {
    pub texts: Vec<Text<'a>>,
    pub splitting: &'a str,
    pub lang: Lang<'a>,
    pub timestamp: u128,
    #[serde(rename = "commonJobParams")]
    pub common_job_params: CommonJobParams<'a>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Text<'a> {
    pub text: &'a str,
    pub request_alternatives: i32,
}

#[derive(Serialize, Debug)]
pub struct PostData<'a> {
    pub jsonrpc: &'a str,
    pub method: &'a str,
    pub id: i64,
    pub params: Params<'a>,
}

impl Default for PostData<'_> {
    fn default() -> Self {
        Self {
            jsonrpc: "2.0",
            method: "LMT_handle_texts",
            id: 0,
            params: Params {
                texts: vec![Text {
                    text: "",
                    request_alternatives: 0,
                }],
                splitting: "newlines",
                lang: Lang {
                    source_lang_user_selected: "auto",
                    target_lang: "ZH",
                },
                timestamp: 0,
                common_job_params: CommonJobParams {
                    was_spoken: false,
                    transcribe_as: "",
                },
            },
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct DeepLResponse {
    pub jsonrpc: String,
    pub id: i64,
    pub result: DeeplResult,
}

#[derive(Deserialize, Debug)]
pub struct DeeplResult {
    pub texts: Vec<TranslatedText>,
    pub lang: String,
    pub lang_is_confident: bool,
    #[serde(rename = "detectedLanguages")]
    pub detected_languages: HashMap<String, f64>,
}

#[derive(Deserialize, Debug)]
pub struct TranslatedText {
    pub alternatives: Vec<Alternative>,
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct Alternative {
    pub text: String,
}

pub fn random_number_id() -> i64 {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let mut rng = rand::rngs::StdRng::seed_from_u64(timestamp);
    let num = rng.gen_range(8300000..8399998);

    return num * 1000;
}

pub fn timestamp_for_i_count(mut i_count: u128) -> u128 {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    if i_count != 0 {
        i_count += 1;
        timestamp - timestamp % i_count + i_count
    } else {
        timestamp
    }
}

pub fn dump_post_data(post_data: PostData) -> String {
    serde_json::to_string(&post_data).unwrap_or_default()
}

pub async fn deepl_translate_request(post_data: String) -> Result<Response, reqwest::Error> {
    let mut headers = HeaderMap::with_capacity(11);
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers.insert("Accept", HeaderValue::from_static("*/*"));
    headers.insert("x-app-os-name", HeaderValue::from_static("iOS"));
    headers.insert("x-app-os-version", HeaderValue::from_static("16.3.0"));
    headers.insert(
        "Accept-Language",
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert(
        "Accept-Encoding",
        HeaderValue::from_static("gzip, deflate, br"),
    );
    headers.insert("x-app-device", HeaderValue::from_static("iPhone13,2"));
    headers.insert(
        "User-Agent",
        HeaderValue::from_static("DeepL-iOS/2.9.1 iOS 16.3.0 (iPhone13,2)"),
    );
    headers.insert("x-app-build", HeaderValue::from_static("510265"));
    headers.insert("x-app-version", HeaderValue::from_static("2.9.1"));
    headers.insert("Connection", HeaderValue::from_static("keep-alive"));

    let client = reqwest::Client::new();
    client
        .post(DEEPL_API)
        .headers(headers)
        .body(post_data)
        .send()
        .await
}

pub async fn deepl_translate<'a>(
    text: &str,
    src_lang: &str,
    target_lang: &str,
) -> Result<DeepLResponse, Response> {
    let count = text
        .as_bytes()
        .iter()
        .fold(timestamp_for_i_count(0), |i_count, e| {
            if *e == 10 {
                i_count + 1
            } else {
                i_count
            }
        });
    let mut post_data = PostData::default();
    let id = random_number_id();
    post_data.id = id;
    post_data.params.timestamp = timestamp_for_i_count(count);
    post_data.params.texts[0].text = text;
    post_data.params.lang.source_lang_user_selected = src_lang;
    post_data.params.lang.target_lang = target_lang;

    let post_data = dump_post_data(post_data);
    let post_data_str = if (id + 5) % 29 == 0 || (id + 3) % 13 == 0 {
        post_data.replace("\"method\":\"", "\"method\" : \"")
    } else {
        post_data.replace("\"method\":\"", "\"method\": \"")
    };
    let resp = deepl_translate_request(post_data_str).await.unwrap();
    match resp.status() {
        StatusCode::OK => {
            let body = resp.json().await.unwrap();
            Ok(body)
        }
        _ => Err(resp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_timestamp() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        println!("{}", timestamp);

        let t = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        println!("{}", t);
    }

    #[test]
    fn test_deepl_translate() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            if let Ok(res) = deepl_translate("hello world", "EN", "ZH").await {
                println!("{:?}", res);
            }
        });
    }
}
