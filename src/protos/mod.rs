//! Generated files are imported from here.
//!
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}

pub mod types {
    pub const MESSAGE_VERSION: u16 = 0;
    pub const MESSAGE_UDP_TUNNEL: u16 = 1;
    pub const MESSAGE_AUTHENTICATE: u16 = 2;
    pub const MESSAGE_PING: u16 = 3;
    pub const MESSAGE_REJECT: u16 = 4;
    pub const MESSAGE_SERVER_SYNC: u16 = 5;
    pub const MESSAGE_CHANNEL_REMOVE: u16 = 6;
    pub const MESSAGE_CHANNEL_STATE: u16 = 7;
    pub const MESSAGE_USER_REMOVE: u16 = 8;
    pub const MESSAGE_USER_STATE: u16 = 9;
    pub const MESSAGE_BAN_LIST: u16 = 10;
    pub const MESSAGE_TEXT_MESSAGE: u16 = 11;
    pub const MESSAGE_PERMISSION_DENIED: u16 = 12;
    pub const MESSAGE_ACL: u16 = 13;
    pub const MESSAGE_QUERY_USERS: u16 = 14;
    pub const MESSAGE_CRYPT_SETUP: u16 = 15;
    pub const MESSAGE_CONTEXT_ACTION_MODIFY: u16 = 16;
    pub const MESSAGE_CONTEXT_ACTION: u16 = 17;
    pub const MESSAGE_USER_LIST: u16 = 18;
    pub const MESSAGE_VOICE_TARGET: u16 = 19;
    pub const MESSAGE_PERMISSION_QUERY: u16 = 20;
    pub const MESSAGE_CODEC_VERSION: u16 = 21;
    pub const MESSAGE_USER_STATS: u16 = 22;
    pub const MESSAGE_REQUEST_BLOB: u16 = 23;
    pub const MESSAGE_SERVER_CONFIG: u16 = 24;
    pub const MESSAGE_SUGGEST_CONFIG: u16 = 25;
}
