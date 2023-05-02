use serde::Deserialize;
use serde::Serialize;

mod conversation_meta;
mod session;
pub use conversation_meta::{
    ConversationMeta, ConversationMetaCreatingError, Result as ConversationMetaCreatingResult,
};
pub use session::{
    ChatError, ChatSession, ConversationStyle, NewBingResponseMessage, Result as SessionResult,
};
mod util;
/// Fields we care about in a Cookie file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CookieInFile {
    /// Name of the cookie.
    pub name: String,
    /// Value of the cookie.
    pub value: String,
}
