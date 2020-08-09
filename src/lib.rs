pub use crate::{
    client::Client,
};

pub mod broker;
pub mod tag_struct;
pub mod client;
pub mod stream;
pub mod command;
pub mod frame;
pub mod sample;
pub mod channel;
pub mod error;

pub const VOLUME_NORMAL: u32 = 0x10000;
pub const PROTOCOL_VERSION: u32 = 8;
pub const INVALID_INDEX: u32 = u32::MAX;
