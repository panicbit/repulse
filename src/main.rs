use anyhow::*;
use tokio::prelude::*;
use tokio::{fs, io, net::UnixStream};
use tokio::time::{self, Duration};
use num_enum::{TryFromPrimitive, IntoPrimitive};

mod tag_struct;
use tag_struct::TagStruct;

mod packet;
use packet::{Command, PacketHeader};

const COMMAND_CHANNEL: u32 = u32::max_value();

#[tokio::main]
async fn main() -> Result<()> {
    let cookie = load_cookie().await?;
    let conn = UnixStream::connect("/run/user/1000/pulse/native").await?;
    let (mut reader, mut writer) = io::split(conn);

    tokio::spawn(async move {
        let frame = Frame::read_from(&mut reader).await.unwrap();

        println!("{:#?}", frame);

        let mut tag_struct = TagStruct::parse(&frame.data).unwrap();
        let packet_header = tag_struct.pop::<PacketHeader>().unwrap();

        println!("{:#?}", packet_header);

        if packet_header.command == Command::Error {
            let error_kind = tag_struct.pop::<ErrorKind>().unwrap();
            println!("Error: {:?}", error_kind);
            return;
        }

        let auth_reply = tag_struct.pop::<AuthReply>().unwrap();

        println!("{:#?}", auth_reply);

        while let Ok(frame) = Frame::read_from(&mut reader).await {
            println!("Received {:#?}", frame);
        }

        println!("Error reading");
    });

    let protocol_version = 7;
    let tag = 42;
    let mut packet = TagStruct::new();

    packet.put(PacketHeader {
        command: Command::Auth,
        tag,
    });

    packet.put(Auth {
        protocol_version,
        cookie,
    });

    Frame::command(&packet)?
        .write_to(&mut writer).await?;

    time::delay_for(Duration::from_millis(1_000)).await;

    Ok(())
}

#[derive(Debug)]
struct Frame {
    channel: u32,
    offset_hi: u32,
    offset_low: u32,
    flags: u32,
    data: Vec<u8>,
}

impl Frame {
    pub fn command(packet: &TagStruct) -> Result<Frame> {
        Ok(Self {
            channel: COMMAND_CHANNEL,
            offset_hi: 0,
            offset_low: 0,
            flags: 0,
            data: packet.to_vec()?,
        })
    }
}

struct Auth {
    protocol_version: u32,
    cookie: Vec<u8>,
}

impl tag_struct::Put for Auth {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.protocol_version);
        tag_struct.put_arbitrary(self.cookie);
    }
}


#[derive(Debug)]
struct AuthReply {
    protocol_version: u32,
}

impl tag_struct::Pop for AuthReply {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        Ok(Self {
            protocol_version: tag_struct.pop_u32().context("Missing version field")?,
        })
    }   
}

impl Frame {
    async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        writer.write_u32(self.data.len() as u32).await?;
        writer.write_u32(self.channel).await?;
        writer.write_u32(self.offset_hi).await?;
        writer.write_u32(self.offset_low).await?;
        writer.write_u32(self.flags).await?;
        writer.write_all(&self.data).await?;
        Ok(())
    }

    async fn read_from<R>(reader: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let size = reader.read_u32().await?;
        let channel = reader.read_u32().await?;
        let offset_hi = reader.read_u32().await?;
        let offset_low = reader.read_u32().await?;
        let flags = reader.read_u32().await?;
        let mut data = vec![0; size as usize];
        reader.read_exact(&mut data).await?;

        Ok(Self {
            channel,
            offset_hi,
            offset_low,
            flags,
            data,
        })
    }
}

async fn load_cookie() -> Result<Vec<u8>> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("pulse/cookie");
    let cookie = fs::read(path).await?;

    Ok(cookie)
}

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
