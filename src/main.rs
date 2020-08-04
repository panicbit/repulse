use anyhow::*;
use futures::prelude::*;
use tokio::{
    fs,
    time::{self, Duration},
};
use crate::{
    broker::Broker,
    tag_struct::{SampleSpec, TagStruct, ChannelMap, ChannelVolume},
    command::{CreatePlaybackStream, SinkRef, CreatePlaybackStreamReply},
    frame::Frame,
    sample::SampleFormat,
    channel::ChannelPosition,
};

pub use crate::{
    client::Client,
};

mod broker;
mod tag_struct;
mod client;
mod command;
mod frame;
mod sample;
mod channel;
mod error;

pub const VOLUME_NORMAL: u32 = 0x10000;
pub const PROTOCOL_VERSION: u32 = 8;
pub const INVALID_INDEX: u32 = u32::MAX;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect().await
        .context("Failed to create client")?;

    let server_info = client.get_server_info().await
        .context("Failed to get server info")?;

    println!("{:#?}", server_info);

    let mut reply = client.broker.send_command(CreatePlaybackStream {
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
        client.broker.send_frame(frame)?;
    }

    Ok(())
}
