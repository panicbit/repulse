use std::{collections::BTreeMap, sync::Arc};
use anyhow::*;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use futures::{pin_mut, channel::{oneshot, mpsc}};
use tokio::io;
use parking_lot::Mutex;
use crate::{
    command::{CommandHeader, Tag, CommandKind, Command},
    tag_struct::{self, TagStruct},
    frame::Frame,
};

type ResponseTransmitter = oneshot::Sender<Result<TagStruct>>;

pub struct Broker {
    response_tx: Arc<Mutex<BTreeMap<Tag, ResponseTransmitter>>>,
    frame_tx: mpsc::Sender<Frame>,
    tag: Tag,
}

impl Broker {
    pub(crate) fn start<S>(stream: S) -> Self
    where S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (mut reader, mut writer) = io::split(stream);
        
        let (frame_tx, frame_rx) = mpsc::channel(1024);

        let response_tx = Arc::new(Mutex::new(BTreeMap::new()));

        let broker = Self {
            response_tx: response_tx.clone(),
            frame_tx,
            tag: 0,
        };

        // Reader
        tokio::spawn({
            let response_tx = response_tx.clone();

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

                        response_tx.lock()
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
                *response_tx.lock() = BTreeMap::new();
            }
        });

        // Writer
        tokio::spawn(async move {
            pin_mut!(frame_rx);

            while let Some(frame) = frame_rx.next().await {
                if let Err(err) = frame.write_to(&mut writer).await {
                    eprintln!("[write]: {:?}", err);
                    // Cancel pending senders
                    *response_tx.lock() = BTreeMap::new();
                }
            }

            eprintln!("[write] Terminating");

            Result::<_>::Ok(())
        });

        broker
    }

    pub(crate) fn send_command<C>(&mut self, command: C) -> Result<impl Future<Output = Result<TagStruct>>>
    where
        C: Command + tag_struct::Put,
    {
        let tag = self.next_tag();
        let (response_tx, response_rx) = oneshot::channel();
        let mut packet = TagStruct::new();

        packet.put(CommandHeader {
            command_kind: C::KIND,
            tag,
        });
    
        packet.put(command);

        // eprintln!("Sending Packet: {:#?}", packet);

        let frame = Frame::command(&packet)?;

        self.response_tx.lock().insert(tag, response_tx);
        self.frame_tx.try_send(frame)
            .map_err(|err| {
                self.response_tx.lock().remove(&tag);
                err
            })?;

        Ok(async move {
            response_rx.await?
        })
    }

    pub(crate) fn send_frame(&mut self, frame: Frame) -> Result<()> {
        self.frame_tx.try_send(frame)?;
        Ok(())
    }

    pub(crate) fn next_tag(&mut self) -> Tag {
        let res = self.tag;
        self.tag += 1;
        res
    }
}
