use base64::engine::general_purpose;
use base64::Engine;
use futures_util::SinkExt;
use futures_util::StreamExt;
use isahc::config::RedirectPolicy;
use isahc::cookies::Cookie;
use isahc::cookies::CookieJar;
use isahc::http::HeaderMap;
use isahc::http::HeaderValue;
use isahc::prelude::Configurable;
use isahc::AsyncReadResponseExt;
use isahc::Request;
use isahc::RequestExt;
use rand::distributions::Slice;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const DELIMITER: u8 = 0x1e;

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
    headers.insert(
        "Sec-WebSocket-Version",
        HeaderValue::from_str("13").unwrap(),
    );
    headers.insert("Connection", HeaderValue::from_str("Upgrade").unwrap());
    headers.insert("Upgrade", HeaderValue::from_str("websocket").unwrap());
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

/// Information of a created conversation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConversationMeta {
    /// used for identify a conversation
    #[serde(rename = "conversationSignature")]
    pub conversation_signature: String,
    /// used for identify a client
    #[serde(rename = "clientId")]
    pub client_id: String,
    /// used for identify a conversation
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
}

/// Fields we care about in a Cookie file.
#[derive(Serialize, Deserialize)]
pub struct CookieInFile {
    /// Name of the cookie.
    pub name: String,
    /// Value of the cookie.
    pub value: String,
}

impl ConversationMeta {
    /// Create a conversation with provided cookies, return the [`ConversationMeta`] of the created conversation.
    async fn create(cookies: &[CookieInFile]) -> ConversationMeta {
        let uri = "https://edgeservices.bing.com/edgesvc/turing/conversation/create"
            .parse()
            .unwrap();
        let cookie_jar: CookieJar = CookieJar::new();
        for cookie_in_file in cookies {
            if let Ok(cookie) = Cookie::builder(&cookie_in_file.name, &cookie_in_file.value).build()
            {
                cookie_jar.set(cookie, &uri).unwrap();
            }
        }
        let mut req = Request::get(&uri)
            .cookie_jar(cookie_jar.clone())
            .redirect_policy(RedirectPolicy::Follow);
        *(req.headers_mut().unwrap()) = create_conversation_headers();
        let mut response = req.body(()).unwrap().send_async().await.unwrap();
        response.json().await.unwrap()
    }
}

fn create_conversation_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authority",
        HeaderValue::from_static("edgeservices.bing.com"),
    );
    headers.insert("accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
    headers.insert(
        "accept-language",
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert("cache-control", HeaderValue::from_static("max-age=0"));
    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            "\"Chromium\";v=\"110\", \"Not A(Brand\";v=\"24\", \"Microsoft Edge\";v=\"110\"",
        ),
    );
    headers.insert("sec-ch-ua-arch", HeaderValue::from_static("\"x86\""));
    headers.insert("sec-ch-ua-bitness", HeaderValue::from_static("\"64\""));
    headers.insert(
        "sec-ch-ua-full-version",
        HeaderValue::from_static("\"110.0.1587.69\""),
    );
    headers.insert("sec-ch-ua-full-version-list", HeaderValue::from_static("\"Chromium\";v=\"110.0.5481.192\", \"Not A(Brand\";v=\"24.0.0.0\", \"Microsoft Edge\";v=\"110.0.1587.69\""));
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert("sec-ch-ua-model", HeaderValue::from_static("\"\""));
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static("\"Windows\""),
    );
    headers.insert(
        "sec-ch-ua-platform-version",
        HeaderValue::from_static("\"15.0.0\""),
    );
    headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("none"));
    headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
    headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
    headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36 Edg/110.0.1587.69"));
    headers.insert("x-edge-shopping-flag", HeaderValue::from_static("1"));
    headers.insert("x-forwarded-for", HeaderValue::from_static("1.1.1.1"));
    headers
}

fn random_hex_string(length: usize) -> String {
    let hex_charactors: Vec<char> = "0123456789abcdef".chars().into_iter().collect();
    rand::thread_rng()
        .sample_iter(Slice::new(&hex_charactors).unwrap())
        .take(length)
        .collect()
}

/// Conversation Style of bing.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ConversationStyle {
    #[serde(rename = "h3relaxedimg")]
    Creative,
    #[serde(rename = "galileo")]
    Balanced,
    #[serde(rename = "h3precise")]
    Precise,
}

impl From<ConversationStyle> for &'static str {
    fn from(val: ConversationStyle) -> Self {
        match val {
            ConversationStyle::Creative => "h3relaxedimg",
            ConversationStyle::Balanced => "galileo",
            ConversationStyle::Precise => "h3precise",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Participant {
    id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NewBingRequestMessage {
    author: &'static str,
    #[serde(rename = "inputMethod")]
    input_method: &'static str,
    text: String,
    #[serde(rename = "messageType")]
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
struct Argument {
    source: &'static str,
    #[serde(rename = "optionsSets")]
    options_sets: [&'static str; 10],
    #[serde(rename = "sliceIds")]
    slice_ids: [&'static str; 3],
    #[serde(rename = "traceId")]
    trace_id: String,
    #[serde(rename = "isStartOfSession")]
    is_start_of_session: bool,
    message: NewBingRequestMessage,
    #[serde(rename = "conversationSignature")]
    conversation_signature: String,
    participant: Participant,
    #[serde(rename = "conversationId")]
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
struct NewBingRequest {
    arguments: [Argument; 1],
    #[serde(rename = "invocationId")]
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

enum NewBingResponse {
    PartResponse,
    FullResponse(NewBingResponseMessage),
    EndOfResponse,
    KeepAlive,
    Unknown,
}

impl<'de> serde::Deserialize<'de> for NewBingResponse {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(d)?;

        Ok(match value.get("type").and_then(Value::as_u64).unwrap() {
            1 => NewBingResponse::PartResponse,
            2 => NewBingResponse::FullResponse(deserialize_newbing_response(value)),
            3 => NewBingResponse::EndOfResponse,
            6 => NewBingResponse::KeepAlive,
            _ => NewBingResponse::Unknown,
        })
    }
}

fn deserialize_newbing_response(value: Value) -> NewBingResponseMessage {
    let content = &value
        .get("item")
        .unwrap()
        .get("messages")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|msg| {
            msg.get("messageType").is_none()
                && msg.get("author").unwrap().as_str().unwrap() == "bot"
        })
        .unwrap();
    let text = content.get("text").unwrap().as_str().unwrap().to_string();
    let suggested_responses = content
        .get("suggestedResponses")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|suggested_response| {
            suggested_response
                .get("text")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let source_attributions = content
        .get("sourceAttributions")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|it| it.get("seeMoreUrl").unwrap().as_str().unwrap().to_string())
        .collect();
    NewBingResponseMessage {
        text,
        suggested_responses,
        source_attributions,
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
    pub async fn create(style: ConversationStyle, cookies: &[CookieInFile]) -> Self {
        let uuid = Uuid::new_v4().hyphenated();
        let uuid = uuid.encode_lower(&mut Uuid::encode_buffer()).to_string();
        Self {
            conversation_meta: ConversationMeta::create(cookies).await,
            invocation_id: 0,
            uuid,
            ip: random_forwarded_ip(),
            style,
        }
    }

    /// Send a message to the session, and return the response.
    pub async fn send_message(&mut self, text: &str) -> Option<NewBingResponseMessage> {
        let mut request = Request::builder()
            .uri("wss://sydney.bing.com/sydney/ChatHub")
            .body(())
            .unwrap();
        *(request.headers_mut()) = headers(&self.uuid, &self.ip);
        let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");
        let (mut write, mut read) = ws_stream.split();
        let mut handshake_message =
            serde_json::to_vec(&json!({"protocol": "json", "version": 1})).unwrap();
        handshake_message.push(DELIMITER);
        let message = Message::Binary(handshake_message);
        write.send(message).await.unwrap();

        let _response = read.next().await.unwrap().unwrap();

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
                let parts = content
                    .split('\u{1e}')
                    .map(|it| it.trim())
                    .filter(|it| !it.is_empty())
                    .collect::<Vec<_>>();
                for part in parts {
                    let response: NewBingResponse = serde_json::from_str(part).unwrap();
                    match response {
                        NewBingResponse::FullResponse(message) => return Some(message),
                        NewBingResponse::EndOfResponse => {
                            break;
                        }
                        NewBingResponse::KeepAlive => {}
                        _ => {}
                    }
                }
            }
        }
        None
    }
}
