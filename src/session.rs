use crate::{conversation_meta, ConversationMeta, CookieInFile};
use base64::{engine::general_purpose, Engine};
use futures_util::{SinkExt, StreamExt};
use rand::{distributions::Slice, Rng};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{http, Message},
};
use uuid::Uuid;

const DELIMITER: u8 = 0x1e;

fn random_hex_string(length: usize) -> String {
    let hex_charactors: Vec<char> = "0123456789abcdef".chars().collect();
    rand::thread_rng()
        .sample_iter(Slice::new(&hex_charactors).unwrap())
        .take(length)
        .collect()
}

fn headers(uuid: &str, forwarded_ip: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("accept", HeaderValue::from_static("application/json"));
    headers.insert(
        "accept-language",
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            "\"Not_A Brand\";v=\"99\", \"Microsoft Edge\";v=\"110\", \"Chromium\";v=\"110\"",
        ),
    );
    headers.insert("sec-ch-ua-arch", HeaderValue::from_static("\"x86\""));
    headers.insert("sec-ch-ua-bitness", HeaderValue::from_static("\"64\""));
    headers.insert(
        "sec-ch-ua-full-version",
        HeaderValue::from_static("\"109.0.1518.78\""),
    );
    headers.insert(
        "sec-ch-ua-full-version-list",
        HeaderValue::from_static(
            "\"Chromium\";v=\"110.0.5481.192\", \"Not A(Brand\";v=\"24.0.0.0\", \"Microsoft Edge\";v=\"110.0.1587.69\"",
        ),
    );
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert("sec-ch-ua-model", HeaderValue::from_static(""));
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static("\"Windows\""),
    );
    headers.insert(
        "sec-ch-ua-platform-version",
        HeaderValue::from_static("\"15.0.0\""),
    );
    headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    headers.insert(
        "x-ms-useragent",
        HeaderValue::from_static(
            "azsdk-js-api-client-factory/1.0.0-beta.1 core-rest-pipeline/1.10.0 OS/Win32",
        ),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://www.bing.com/search?q=Bing+AI&showconv=1&FORM=hpcodx"),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("origin-when-cross-origin"),
    );

    headers.insert(
        "x-ms-client-request-id",
        HeaderValue::from_str(uuid).unwrap(),
    );
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_str(forwarded_ip).unwrap(),
    );
    let websocket_key = random_hex_string(16);
    let websocket_key_base64 = general_purpose::STANDARD.encode(websocket_key);
    headers.insert(
        "Sec-websocket-key",
        HeaderValue::from_str(&websocket_key_base64).unwrap(),
    );
    headers.insert("Sec-WebSocket-Version", HeaderValue::from_static("13"));
    headers.insert("Connection", HeaderValue::from_static("Upgrade"));
    headers.insert("Upgrade", HeaderValue::from_static("websocket"));
    headers.insert("Host", HeaderValue::from_static("sydney.bing.com"));
    headers
}

fn random_forwarded_ip() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "13.{}.{}.{}",
        rng.gen_range(104u8..=107u8),
        rng.gen_range(0u8..=255),
        rng.gen_range(0u8..=255)
    )
}

/// Conversation Style of bing.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ConversationStyle {
    #[serde(rename = "h3imaginative")]
    Creative,
    #[serde(rename = "galileo")]
    Balanced,
    #[serde(rename = "h3precise")]
    Precise,
}

impl From<ConversationStyle> for &'static str {
    fn from(val: ConversationStyle) -> Self {
        match val {
            ConversationStyle::Creative => "h3imaginative",
            ConversationStyle::Balanced => "galileo",
            ConversationStyle::Precise => "h3precise",
        }
    }
}

/// A session represent a chat with bing.
/// It implements `Serialize` and `Deserialize`
/// Thus can be dumped to/load from external storage to pause and continue a chat.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatSession {
    conversation_meta: ConversationMeta,
    invocation_id: usize,
    uuid: String,
    ip: String,
    style: ConversationStyle,
}

/// Response provided by bing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBingResponseMessage {
    /// text content of the response.
    pub text: String,
    /// suggested responses of the response.
    pub suggested_responses: Vec<String>,
    /// source attributions of the response.
    pub source_attributions: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct NewBingRequestMessage {
    author: &'static str,
    input_method: &'static str,
    text: String,
    message_type: &'static str,
}

impl NewBingRequestMessage {
    fn new(text: String) -> Self {
        Self {
            author: "user",
            input_method: "Keyboard",
            text,
            message_type: "Chat",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Participant {
    id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Argument {
    source: &'static str,
    options_sets: [&'static str; 10],
    slice_ids: [&'static str; 3],
    trace_id: String,
    is_start_of_session: bool,
    message: NewBingRequestMessage,
    conversation_signature: String,
    participant: Participant,
    conversation_id: String,
}

impl Argument {
    pub fn new(
        conversation_meta: ConversationMeta,
        style: ConversationStyle,
        is_start_of_session: bool,
        text: &str,
    ) -> Self {
        Self {
            source: "cib",
            options_sets: [
                "nlu_direct_response_filter",
                "deepleo",
                "disable_emoji_spoken_text",
                "responsible_ai_policy_235",
                "enablemm",
                style.into(),
                "dtappid",
                "cricinfo",
                "cricinfov2",
                "dv3sugg",
            ],
            slice_ids: ["222dtappid", "225cricinfo", "224locals0"],
            trace_id: random_hex_string(32),
            is_start_of_session,
            message: NewBingRequestMessage::new(text.to_string()),
            conversation_signature: conversation_meta.conversation_signature.to_string(),
            participant: Participant {
                id: conversation_meta.client_id.to_string(),
            },
            conversation_id: conversation_meta.conversation_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct NewBingRequest {
    arguments: [Argument; 1],
    invocation_id: String,
    target: &'static str,
    #[serde(rename = "type")]
    message_type: u8,
}

impl NewBingRequest {
    fn new(
        conversation_meta: ConversationMeta,
        style: ConversationStyle,
        invocation_id: usize,
        text: &str,
    ) -> Self {
        Self {
            arguments: [Argument::new(
                conversation_meta,
                style,
                invocation_id == 0,
                text,
            )],
            invocation_id: format!("{invocation_id}"),
            target: "chat",
            message_type: 4,
        }
    }
}

#[derive(Debug, Clone)]
enum SignalRNewBingResponse {
    Invocation,
    StreamItem(NewBingResponseMessage),
    EndOfResponse,
    Ping,
    Unknown,
}

impl<'de> serde::Deserialize<'de> for SignalRNewBingResponse {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let value = Value::deserialize(d)?;

        Ok(match value.get("type").and_then(Value::as_u64).unwrap() {
            1 => SignalRNewBingResponse::Invocation,
            2 => SignalRNewBingResponse::StreamItem(deserialize_newbing_response(value).unwrap()),
            3 => SignalRNewBingResponse::EndOfResponse,
            6 => SignalRNewBingResponse::Ping,
            _ => SignalRNewBingResponse::Unknown,
        })
    }
}

fn deserialize_newbing_response(value: Value) -> Result<NewBingResponseMessage> {
    let content = &value
        .get("item")
        .ok_or(ChatError::GetFieldError {
            object_name: "newbing_response",
            field_name: "item",
        })?
        .get("messages")
        .ok_or(ChatError::GetFieldError {
            object_name: "newbing_response.item",
            field_name: "messages",
        })?
        .as_array()
        .ok_or(ChatError::FieldTypeError {
            object_name: "newbing_response.item",
            field_name: "messages",
            expected_type: "str",
        })?
        .iter()
        .find(|msg| {
            msg.get("messageType").is_none()
                && msg
                    .get("author")
                    .and_then(|author| author.as_str())
                    .map(|it| it == "bot")
                    .unwrap_or(false)
        })
        .ok_or(ChatError::FieldTypeError {
            object_name: "newbing_response.item",
            field_name: "messages[{author == bot, messageType != null}]",
            expected_type: "str",
        })?;
    let text = content
        .get("text")
        .ok_or(ChatError::GetFieldError {
            object_name: "newbing_response.item.messages[{author == bot, messageType != null}]",
            field_name: "text",
        })?
        .as_str()
        .ok_or(ChatError::FieldTypeError {
            object_name: "newbing_response.item.messages[{author == bot, messageType != null}]",
            field_name: "text",
            expected_type: "str",
        })?
        .to_string();
    let suggested_responses = content
        .get("suggestedResponses")
        .ok_or(ChatError::GetFieldError {
            object_name: "newbing_response.item.messages[{author == bot, messageType != null}]",
            field_name: "suggestedResponses",
        })?
        .as_array()
        .ok_or(ChatError::FieldTypeError {
            object_name: "newbing_response.item.messages[{author == bot, messageType != null}]",
            field_name: "suggestedResponses",
            expected_type: "array",
        })?
        .iter()
        .map(|suggested_response|{
            Ok(suggested_response
                .get("text")
                .ok_or(ChatError::GetFieldError {
                    object_name: "newbing_response.item.messages[{author == bot, messageType != null}].suggestedResponses",
                    field_name: "text",
                })?
                .as_str()
                .ok_or(ChatError::FieldTypeError {
                    object_name: "newbing_response.item.messages[{author == bot, messageType != null}].suggestedResponses",
                    field_name: "text",
                    expected_type: "str",
                })?
                .to_string())
        })
        .collect::<Result<_>>()?;
    let source_attributions = content
        .get("sourceAttributions")
        .ok_or(ChatError::GetFieldError {
            object_name: "newbing_response.item.messages[{author == bot, messageType != null}]",
            field_name: "sourceAttributions",
        })?
        .as_array()
        .ok_or(ChatError::FieldTypeError {
            object_name: "newbing_response.item.messages[{author == bot, messageType != null}]",
            field_name: "sourceAttributions",
            expected_type: "array",
        })?
        .iter()
        .map(|it| {
            Ok(it
                .get("seeMoreUrl")
                .ok_or(ChatError::GetFieldError {
                    object_name:
                        "newbing_response.item.messages[{author == bot, messageType != null}].sourceAttributions",
                    field_name: "seeMoreUrl",
                })?
                .as_str()
                .ok_or(ChatError::FieldTypeError {
                    object_name:
                        "newbing_response.item.messages[{author == bot, messageType != null}].sourceAttributions",
                    field_name: "seeMoreUrl",
                    expected_type: "str",
                })?
                .to_string())
        })
        .collect::<Result<_>>()?;
    Ok(NewBingResponseMessage {
        text,
        suggested_responses,
        source_attributions,
    })
}

impl ChatSession {
    pub fn new(
        conversation_meta: ConversationMeta,
        style: ConversationStyle,
        invocation_id: usize,
        uuid: String,
        ip: String,
    ) -> Self {
        Self {
            style,
            conversation_meta,
            invocation_id,
            uuid,
            ip,
        }
    }

    /// Create a new [`ChatSession`] from cookies.
    pub async fn create(
        style: ConversationStyle,
        cookies: &[CookieInFile],
    ) -> conversation_meta::Result<Self> {
        let uuid = Uuid::new_v4().hyphenated();
        let uuid = uuid.encode_lower(&mut Uuid::encode_buffer()).to_string();
        Ok(Self {
            conversation_meta: ConversationMeta::create(cookies).await?,
            invocation_id: 0,
            uuid,
            ip: random_forwarded_ip(),
            style,
        })
    }

    /// Send a message to the session, and return the response.
    pub async fn send_message(&mut self, text: &str) -> Result<NewBingResponseMessage> {
        let mut request = http::Request::builder()
            .uri("wss://sydney.bing.com/sydney/ChatHub")
            .body(())
            .unwrap();
        *(request.headers_mut()) = headers(&self.uuid, &self.ip);
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|_| ChatError::Network)?;
        let (mut write, mut read) = ws_stream.split();
        let mut handshake_message =
            serde_json::to_vec(&json!({"protocol": "json", "version": 1})).unwrap();
        handshake_message.push(DELIMITER);
        let message = Message::Binary(handshake_message);
        write.send(message).await.map_err(|_| ChatError::Network)?;

        let _response = read.next().await.unwrap().map_err(|_| ChatError::Network)?;

        let mut alive_message = serde_json::to_vec(&json!({"type": 6})).unwrap();
        alive_message.push(DELIMITER);
        let message = Message::Binary(alive_message);
        write.send(message).await.unwrap();

        let msg = NewBingRequest::new(
            self.conversation_meta.clone(),
            self.style,
            self.invocation_id,
            text,
        );
        let mut question_message = serde_json::to_vec(&msg).unwrap();
        question_message.push(DELIMITER);
        let message = Message::Binary(question_message);
        write.send(message).await.unwrap();
        self.invocation_id += 1;

        while let Some(Ok(response)) = read.next().await {
            if let Message::Text(content) = response {
                let signal_r_packages = content
                    .split('\u{1e}')
                    .map(|it| it.trim())
                    .filter(|it| !it.is_empty())
                    .collect::<Vec<_>>();
                for signal_r_package in signal_r_packages {
                    let response: SignalRNewBingResponse = serde_json::from_str(signal_r_package)?;
                    match response {
                        SignalRNewBingResponse::StreamItem(message) => return Ok(message),
                        SignalRNewBingResponse::EndOfResponse => {
                            break;
                        }
                        SignalRNewBingResponse::Ping => {
                            let mut alive_message =
                                serde_json::to_vec(&json!({"type": 6})).unwrap();
                            alive_message.push(DELIMITER);
                            let message = Message::Binary(alive_message);
                            write.send(message).await.map_err(|_| ChatError::Network)?;
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(ChatError::NoFullResponseFound)
    }
}

#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Failed to get field {field_name} from {object_name}")]
    GetFieldError {
        object_name: &'static str,
        field_name: &'static str,
    },
    #[error("{object_name}.{field_name} should be of type {expected_type}")]
    FieldTypeError {
        object_name: &'static str,
        field_name: &'static str,
        expected_type: &'static str,
    },
    #[error("Failed to send chat request")]
    Network,
    #[error("Failed to parse chat response")]
    ParseRespond(#[from] serde_json::Error),
    #[error("No full response received")]
    NoFullResponseFound,
}

pub type Result<T> = std::result::Result<T, ChatError>;
