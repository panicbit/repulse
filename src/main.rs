use anyhow::*;
use tokio::{fs, io, net::UnixStream};
use tokio::time::{self, Duration};

mod tag_struct;
use tag_struct::TagStruct;

mod command;
use command::{CommandHeader, CommandKind};

mod frame;
use frame::Frame;

mod error;
use error::ErrorKind;

pub const VOLUME_NORMAL: u32 = 0x10000;

#[tokio::main]
async fn main() -> Result<()> {
    let cookie = load_cookie().await?;
    let conn = UnixStream::connect("/run/user/1000/pulse/native").await?;
    let (mut reader, mut writer) = io::split(conn);

    tokio::spawn(async move {
        let frame = Frame::read_from(&mut reader).await.unwrap();

        // println!("{:#?}", frame);

        let mut packet = TagStruct::parse(&frame.data).unwrap();
        let command_header = packet.pop::<CommandHeader>().unwrap();

        println!("{:#?}", command_header);

        if command_header.command_kind == CommandKind::Error {
            let error_kind = packet.pop::<ErrorKind>().unwrap();
            println!("Error: {:?}", error_kind);
            return;
        }

        let auth_reply = packet.pop::<command::AuthReply>().unwrap();

        println!("{:#?}", auth_reply);

        while let Ok(frame) = Frame::read_from(&mut reader).await {
            let mut packet = TagStruct::parse(&frame.data).unwrap();
            let command_header = packet.pop::<CommandHeader>().unwrap();

            println!("Received header: {:#?}", command_header);
            println!("TagList: {:#?}", packet);
        }

        println!("Error reading");
    });

    let protocol_version = 8;
    let mut tag = 42;
        
    {
        let mut packet = TagStruct::new();

        packet.put(CommandHeader {
            command_kind: CommandKind::Auth,
            tag,
        });

        tag += 1;

        packet.put(command::Auth {
            protocol_version,
            cookie,
        });

        Frame::command(&packet)?
            .write_to(&mut writer).await?;
    }

    {
        let mut packet = TagStruct::new();

        packet.put(CommandHeader {
            command_kind: CommandKind::PlaySample,
            tag,
        });

        tag += 1;

        packet.put(command::PlaySample {
            sink_index: 0,
            sink_name: None,
            volume: VOLUME_NORMAL / 2,
            sample_name: "screen-capture".into(),
        });

        Frame::command(&packet)?
            .write_to(&mut writer).await?;
    }

    time::delay_for(Duration::from_millis(1_000)).await;

    Ok(())
}

async fn load_cookie() -> Result<Vec<u8>> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("pulse/cookie");
    let cookie = fs::read(path).await?;

    Ok(cookie)
}
