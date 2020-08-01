use anyhow::*;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use futures::{pin_mut, channel::{oneshot, mpsc}};
use tokio::{fs, io, net::UnixStream};
use tokio::time::{self, Duration};
use parking_lot::Mutex;

mod tag_struct;
use tag_struct::{SampleSpec, TagStruct, ChannelMap, ChannelVolume};

mod command;
use command::{CommandHeader, Tag, Command, AuthReply, CreatePlaybackStream, SinkRef, CreatePlaybackStreamReply, CommandKind};

mod frame;
use frame::Frame;

mod sample;
mod channel;

mod error;
use std::{sync::Arc, collections::BTreeMap};
use sample::SampleFormat;
use channel::ChannelPosition;

pub const VOLUME_NORMAL: u32 = 0x10000;
pub const PROTOCOL_VERSION: u32 = 8;
pub const INVALID_INDEX: u32 = u32::MAX;

#[tokio::main]
async fn main() -> Result<()> {
    let cookie = load_cookie().await?;
    let conn = UnixStream::connect("/run/user/1000/pulse/native").await?;

    let mut broker = Broker::start(conn);

    let mut reply = broker.send_command(command::Auth {
        protocol_version: PROTOCOL_VERSION,
        cookie,
    })?.await?;

    let reply = reply.pop::<AuthReply>()?;

    println!("{:#?}", reply);

    let mut reply = broker.send_command(CreatePlaybackStream {
        name: "🦀 Repulse - Native Rust Client 🦀".into(),
        sample_spec: SampleSpec {
            format: SampleFormat::S16LE,
            channels: 2,
            rate: 44100,
        },
        channel_map: ChannelMap {
            positions: vec![
                ChannelPosition::FrontLeft,
                ChannelPosition::FrontRight,
            ],
        },
        sink_ref: SinkRef::index(0),
        max_length: u32::MAX,
        corked: false,
        t_length: u32::MAX,
        prebuf: u32::MAX,
        min_req: u32::MAX,
        sync_id: 0,
        volume: ChannelVolume {
            volumes: vec![
                VOLUME_NORMAL / 2,
                VOLUME_NORMAL / 2,
            ],
        },

    })?.await?;

    let reply = reply.pop::<CreatePlaybackStreamReply>()?;

    println!("{:#?}", reply);

    let data = include_bytes!("/tmp/audio.raw");

    let bytes_per_second = 2 * 2 * 44100;
    let mut interval = time::interval(Duration::from_secs(1));

    for chunk in data.chunks(bytes_per_second) {
        let frame = Frame {
            channel: reply.index,
            offset_hi: 0,
            offset_low: 0,
            flags: 0,
            data: chunk.into(),
        };

        interval.next().await;
        broker.send_frame(frame)?;
    }

    Ok(())
}

pub type Handler = Box<dyn FnOnce(TagStruct) -> Result<()>>;

pub struct Broker {
    tag_struct_tx: Arc<Mutex<BTreeMap<Tag, oneshot::Sender<Result<TagStruct>>>>>,
    frame_tx: mpsc::Sender<Frame>,
    tag: Tag,
}

impl Broker {
    fn start<S>(stream: S) -> Self
    where S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (mut reader, mut writer) = io::split(stream);
        
        let (frame_tx, frame_rx) = mpsc::channel(1024);

        let tag_struct_tx = Arc::new(Mutex::new(BTreeMap::new()));

        let broker = Self {
            tag_struct_tx: tag_struct_tx.clone(),
            frame_tx,
            tag: 0,
        };

        // Reader
        tokio::spawn({
            let tag_struct_tx = tag_struct_tx.clone();

            async move {
                let read_loop = async {
                    loop {
                        let frame = Frame::read_from(&mut reader).await?;

                        if !frame.is_command_frame() {
                            eprintln!("Received non-command frame (TODO)");
                            continue;
                        }

                        let mut packet = TagStruct::parse(&frame.data)?;
                        let command_header = packet.pop::<CommandHeader>()?;

                        if !command_header.command_kind.is_reply() {
                            match command_header.command_kind {
                                CommandKind::Request => {
                                    eprintln!("[TODO] Received REQUEST: {:#?}", packet);
                                    continue;
                                },
                                _ => {
                                    eprintln!("[TODO] Received unhandled {:?}: {:#?}", command_header.command_kind, packet);
                                    continue;
                                }
                            }
                        }

                        tag_struct_tx.lock()
                            .remove(&command_header.tag)
                            .context("Unknown tag received")?
                            .send(Ok(packet))
                            .ok();
                    }

                    #[allow(unreachable_code)]
                    Result::<_>::Ok(())
                };

                if let Err(err) = read_loop.await {
                    eprintln!("[read]: {:?}", err);
                }

                // Cancel pending senders
                *tag_struct_tx.lock() = BTreeMap::new();
            }
        });

        // Writer
        tokio::spawn(async move {
            pin_mut!(frame_rx);

            while let Some(frame) = frame_rx.next().await {
                if let Err(err) = frame.write_to(&mut writer).await {
                    eprintln!("[write]: {:?}", err);
                    // Cancel pending senders
                    *tag_struct_tx.lock() = BTreeMap::new();
                }
            }

            eprintln!("[write] Terminating");

            Result::<_>::Ok(())
        });

        broker
    }

    fn send_command<C>(&mut self, command: C) -> Result<impl Future<Output = Result<TagStruct>>>
    where
        C: Command + tag_struct::Put,
    {
        let tag = self.next_tag();
        let (tag_struct_tx, tag_struct_rx) = oneshot::channel();
        let mut packet = TagStruct::new();

        packet.put(CommandHeader {
            command_kind: C::KIND,
            tag,
        });
    
        packet.put(command);

        println!("Sending Packet: {:#?}", packet);

        let frame = Frame::command(&packet)?;

        self.tag_struct_tx.lock().insert(tag, tag_struct_tx);
        self.frame_tx.try_send(frame)
            .map_err(|err| {
                self.tag_struct_tx.lock().remove(&tag);
                err
            })?;

        Ok(async move {
            tag_struct_rx.await?
        })
    }

    fn send_frame(&mut self, frame: Frame) -> Result<()> {
        self.frame_tx.try_send(frame)?;
        Ok(())
    }

    fn next_tag(&mut self) -> Tag {
        let res = self.tag;
        self.tag += 1;
        res
    }
}

async fn load_cookie() -> Result<Vec<u8>> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("pulse/cookie");
    let cookie = fs::read(path).await?;

    Ok(cookie)
}
