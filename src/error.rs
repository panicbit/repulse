use num_enum::{TryFromPrimitive, IntoPrimitive};
use anyhow::*;
use crate::tag_struct::{self, TagStruct};

#[derive(Debug, TryFromPrimitive, IntoPrimitive, Copy, Clone)]
#[repr(u32)]
pub enum ErrorKind {
    /// No error
    Ok,
    /// Access failure
    Access,
    /// Unknown command
    Command,
    /// Invalid argument
    Invalid,
    /// Entity exists
    Exist,
    /// No such entity
    NoEntity,
    /// Connection refused
    ConnectionRefused,
    /// Protocol error
    Protocol,
    /// Timeout
    Timeout,
    /// No authentication key
    AuthKey,
    /// Internal error
    Internal,
    /// Connection terminated
    ConnectionTerminated,
    /// Entity killed
    Killed,
    /// Invalid server
    InvalidServer,
    /// Module initialization failed
    ModInitFailed,
    /// Bad state
    BadState,
    /// No data
    NoData,
    /// Incompatible protocol version
    Version,
    /// Data too large
    TooLarge,
    /// Operation not supported \since 0.9.5
    NotSupported,
    /// The error code was unknown to the client
    Unknown,
    /// Extension does not exist. \since 0.9.12
    NoExtension,
    /// Obsolete functionality. \since 0.9.15
    Obsolete,
    /// Missing implementation. \since 0.9.15
    NotImplemented,
    /// The caller forked without calling execve() and tried to reuse the context. \since 0.9.15
    Forked,
    /// An IO error happened. \since 0.9.16
    Io,
    /// Device or resource busy. \since 0.9.17
    Busy,
}

impl tag_struct::Pop for ErrorKind {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        let error_kind = tag_struct.pop_u32()?;
        let error_kind = Self::try_from_primitive(error_kind)?;

        Ok(error_kind)
    }
}
