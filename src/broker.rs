use std::{collections::BTreeMap, sync::{Weak, Arc}};
use anyhow::*;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use futures::{pin_mut, channel::{oneshot, mpsc}, future::AbortHandle};
use tokio::io;
use parking_lot::Mutex;
use crate::{
    command::{CommandHeader, Tag, CommandKind, Command},
    tag_struct::{self, TagStruct},
    frame::Frame,
};
use future::Abortable;

type ResponseTransmitter = oneshot::Sender<Result<TagStruct>>;

#[derive(Clone)]
pub struct Broker {
    data: Arc<Mutex<BrokerData>>,
}

impl Broker {
    pub(crate) fn start<S>(stream: S) -> Self
    where S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (reader, writer) = io::split(stream);
        let (frame_tx, frame_rx) = mpsc::channel(1024);

        let (read_loop_abort_handle, read_loop_abort_registration) = AbortHandle::new_pair();
        let (write_loop_abort_handle, write_loop_abort_registration) = AbortHandle::new_pair();

        let data = Arc::new(Mutex::new(BrokerData {
            response_tx: BTreeMap::new(),
            frame_tx,
            tag: 0,
            read_loop_abort_handle,
            write_loop_abort_handle,
        }));
        let weak_data = Arc::downgrade(&data);

        let read_loop = Self::read_loop(weak_data.clone(), reader);
        let read_loop = guard_loop(read_loop, weak_data.clone());
        let read_loop = Abortable::new(read_loop, read_loop_abort_registration);

        let write_loop = Self::write_loop(weak_data.clone(), writer, frame_rx);
        let write_loop = guard_loop(write_loop, weak_data);
        let write_loop = Abortable::new(write_loop, write_loop_abort_registration);

        tokio::spawn(read_loop);
        tokio::spawn(write_loop);

        Self {
            data,
        }
    }

    pub(crate) fn send_command<C>(&self, command: C) -> Result<impl Future<Output = Result<TagStruct>>>
    where
        C: Command + tag_struct::Put,
    {
        self.data.lock().send_command(command)
    }

    pub(crate) fn send_frame(&self, frame: Frame) -> Result<()> {
        self.data.lock().send_frame(frame)
    }

    async fn read_loop<R>(data: Weak<Mutex<BrokerData>>, mut reader: R) -> Result<()>
    where
        R: AsyncRead + Unpin,
    {
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

            if let Some(data) = data.upgrade() {
                data.lock().response_tx
                    .remove(&command_header.tag)
                    .context("Unknown tag received")?
                    .send(Ok(packet))
                    .ok();
            }
        }
    }

    async fn write_loop<W>(_data: Weak<Mutex<BrokerData>>, mut writer: W, frame_rx: mpsc::Receiver<Frame>) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        pin_mut!(frame_rx);

        while let Some(frame) = frame_rx.next().await {
            frame.write_to(&mut writer).await?;
        }

        eprintln!("[write] Terminating");

        Ok(())
    }
}

struct BrokerData {
    response_tx: BTreeMap<Tag, ResponseTransmitter>,
    frame_tx: mpsc::Sender<Frame>,
    tag: Tag,
    read_loop_abort_handle: AbortHandle,
    write_loop_abort_handle: AbortHandle,
}

impl BrokerData {
    fn stop(&mut self) {
        self.read_loop_abort_handle.abort();
        self.write_loop_abort_handle.abort();
        self.response_tx = BTreeMap::new();
    }

    fn send_command<C>(&mut self, command: C) -> Result<impl Future<Output = Result<TagStruct>>>
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

        self.response_tx.insert(tag, response_tx);
        self.frame_tx.try_send(frame)
            .map_err(|err| {
                self.response_tx.remove(&tag);
                err
            })?;

        Ok(async move {
            response_rx.await?
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

impl Drop for BrokerData {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn guard_loop(loop_future: impl Future<Output = Result<()>>, data: Weak<Mutex<BrokerData>>) {
    let result = loop_future.await;

    if let Err(err) = result {
        eprintln!("[debug] Loop terminating: {}", err);
    }

    if let Some(data) = data.upgrade() {
        data.lock().stop();
    }
}
