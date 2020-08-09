use anyhow::*;
use tokio::fs;
use tokio::net::UnixStream;
use futures::channel::oneshot;
use crate::{command, PROTOCOL_VERSION, tag_struct, broker::{SendFrame, start_broker}, frame::Frame, stream::PlaybackStream, VOLUME_NORMAL};
use command::{ServerInfo, Command, AuthReply, CommandHeader, Tag, CommandKind, CreatePlaybackStream, CreatePlaybackStreamReply, SinkRef};
use std::{collections::{btree_map, BTreeMap}, sync::Arc, mem};
use tokio::sync::Mutex;
use tag_struct::{SampleSpec, TagStruct, ChannelMap, ChannelVolume};

#[derive(Clone)]
pub struct Client {
    inner: Arc<Mutex<InnerClient>>,
}

impl Client {
    pub async fn connect() -> Result<Self> {
        let cookie = load_cookie().await
            .context("Loading cookie failed")?;
        let conn = UnixStream::connect("/run/user/1000/pulse/native").await
            .context("Connecting to server failed")?;

        let inner = InnerClient {
            send_frame: Box::new(|_| panic!()), // TODO: Get rid of this
            next_tag: 0,
            reply_senders: BTreeMap::new(),
            sync_id: 0,
        };
        let inner = Arc::new(Mutex::new(inner));
        let client = Self { inner };

        let (send_frame, abort_handle) = {
            let client = client.clone();

            start_broker(conn, move |frame| {
                let client = client.clone();
                
                async move {
                    client.on_frame(frame).await
                }
            })
        };

        // TODO: Get rid of this
        client.inner.lock().await.send_frame = send_frame;

        client.send_command::<_, AuthReply>(command::Auth {
            protocol_version: PROTOCOL_VERSION,
            cookie,
        }).await?;

        Ok(client)
    }

    pub async fn get_server_info(&self) -> Result<ServerInfo> {
        self.send_command::<_, ServerInfo>(command::GetServerInfo).await
    }

    pub(crate) async fn send_command<C, R>(&self, command: C) -> Result<R>
    where
        C: Command + tag_struct::Put,
        R: tag_struct::Pop,
    {
        let tag = self.next_tag().await;
        let mut packet = TagStruct::new();
        packet.put(CommandHeader {
            command_kind: C::KIND,
            tag,
        });
        packet.put(command);

        eprintln!("Prepared packet: {:#?}", packet);

        let frame = Frame::command(&packet)
            .context("Failed to serialize packet")?;
        

        let reply_rx = self.register_reply(tag).await
            .context("Failed to register reply")?;

        eprintln!("Sending frame");
        self.send_frame(frame)
            .await
            .context("Failed to send frame")?;

        eprintln!("Awaiting reply");
        let mut reply = reply_rx.await
            .context("Failed to receive reply (sender gone)")?
            .context("Failed to receive reply")?;

        eprintln!("Got reply: {:#?}", reply);

        let parsed_reply = reply.pop::<R>()?;

        if !reply.is_empty() {
            eprintln!("Incomplete packet parse. Remaining fields: {:#?}", reply);
        }


        Ok(parsed_reply)
    }

    pub async fn create_playback_stream(
        &self,
        name: impl Into<String>,
        sample_spec: SampleSpec,
        channel_map: ChannelMap,
    ) -> Result<PlaybackStream> {
        let volume = ChannelVolume {
            volumes: channel_map.positions.iter().map(|_| VOLUME_NORMAL).collect(),
        };

        let request = CreatePlaybackStream {
            name: name.into(),
            sample_spec,
            channel_map,
            sink_ref: SinkRef::name("@DEFAULT_SINK@"),
            max_length: u32::MAX,
            corked: false,
            t_length: u32::MAX,
            prebuf: u32::MAX,
            min_req: u32::MAX,
            sync_id: self.next_sync_id().await,
            volume,

        };

        let reply = self.send_command::<_, CreatePlaybackStreamReply>(request).await?;

        let channel = reply.index;
        let stream = PlaybackStream::new(self, channel);

        Ok(stream)
    }

    pub(crate) async fn send_frame(&self, frame: Frame) -> Result<()> {
        self.inner.lock().await.send_frame(frame).await
    }

    async fn on_frame(&self, frame: Result<Frame>) {
        let mut inner = self.inner.lock().await;
        if let Err(err) = inner.on_frame(frame) {
            inner.handle_fatal_error(err);
        }
    }

    async fn next_tag(&self) -> Tag {
        self.inner.lock().await.next_tag()
    }

    async fn next_sync_id(&self) -> u32 {
        self.inner.lock().await.next_sync_id()
    }

    async fn register_reply(&self, tag: Tag) -> Result<oneshot::Receiver<Result<TagStruct>>> {
        self.inner.lock().await.register_reply(tag)
    }
}

async fn load_cookie() -> Result<Vec<u8>> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("pulse/cookie");
    let cookie = fs::read(path).await?;

    Ok(cookie)
}

struct InnerClient {
    send_frame: SendFrame,
    next_tag: Tag,
    reply_senders: BTreeMap<Tag, oneshot::Sender<Result<TagStruct>>>,
    sync_id: u32,
}

impl InnerClient {
    fn on_frame(&mut self, frame: Result<Frame>) -> Result<()> {
        eprintln!("Entering on_frame");

        let frame = frame.context("Failed to handle frame")?;

        if !frame.is_command_frame() {
            eprintln!("Received non-command frame (TODO)");
            return Ok(());
        }

        let mut packet = TagStruct::parse(&frame.data)?;
        let command_header = packet.pop::<CommandHeader>()?;

        if !command_header.command_kind.is_reply() {
            match command_header.command_kind {
                CommandKind::Request => {
                    eprintln!("[TODO] Received REQUEST: {:#?}", packet);
                    return Ok(());
                },
                _ => {
                    eprintln!("[TODO] Received unhandled {:?}: {:#?}", command_header.command_kind, packet);
                    return Ok(());
                }
            }
        }

        let tag = command_header.tag;
        self.reply(tag, packet);

        Ok(())
    }

    async fn send_frame(&mut self, frame: Frame) -> Result<()> {
        let fut = (self.send_frame)(frame);
        fut.await
    }

    fn next_tag(&mut self) -> Tag {
        let res = self.next_tag;
        self.next_tag += 1;
        // Skip the tag that is used for server replies and wrap around
        self.next_tag %= Tag::MAX;
        res
    }

    fn next_sync_id(&mut self) -> u32 {
        let res = self.next_tag;
        self.next_tag += 1;
        // wrap around
        self.next_tag %= Tag::MAX;
        res
    }

    fn register_reply(&mut self, tag: Tag) -> Result<oneshot::Receiver<Result<TagStruct>>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        match self.reply_senders.entry(tag) {
            btree_map::Entry::Occupied(_) => bail!("BUG: duplicate reply entry"),
            btree_map::Entry::Vacant(entry) => entry.insert(reply_tx),
        };

        Ok(reply_rx)
    }

    fn reply(&mut self, tag: Tag, packet: TagStruct) -> Result<()> {
        eprintln!("Reply.");
        self.reply_senders
            .remove(&tag)
            .with_context(|| format!("Received reply with unknown tag {}", tag))?
            .send(Ok(packet))
            .ok();

        Ok(())
    }

    fn handle_fatal_error(&mut self, err: Error) {
        // TODO: improve error handling
        let reply_senders = mem::replace(&mut self.reply_senders, BTreeMap::new());

        for (_tag, reply_tx) in reply_senders {
            reply_tx.send(Err(anyhow!("{}", err)));
        }
    }
}
