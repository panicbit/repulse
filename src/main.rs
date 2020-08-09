use anyhow::*;
use futures::prelude::*;
use tokio::{fs, time::{self, Duration}};
use crate::{
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
    let client = Client::connect().await
        .context("Failed to create client")?;

    let server_info = client.get_server_info().await
        .context("Failed to get server info")?;

    println!("{:#?}", server_info);

    let data = fs::read("/tmp/audio.raw").await?;
    let sample_rate: usize = 44_100;
    let num_channels: usize = 2;

    let reply = client.send_command::<_, CreatePlaybackStreamReply>(CreatePlaybackStream {
        name: "🦀 Repulse - Native Rust Client 🦀".into(),
        sample_spec: SampleSpec {
            format: SampleFormat::S16LE,
            channels: num_channels as u8,
            rate: sample_rate as u32,
        },
        channel_map: ChannelMap {
            positions: vec![
                ChannelPosition::FrontLeft,
                ChannelPosition::FrontRight,
            ],
        },
        sink_ref: SinkRef::name("@DEFAULT_SINK@"),
        max_length: u32::MAX,
        corked: false,
        t_length: u32::MAX,
        prebuf: u32::MAX,
        min_req: u32::MAX,
        sync_id: 0,
        volume: ChannelVolume {
            volumes: vec![
                VOLUME_NORMAL,
                VOLUME_NORMAL,
            ],
        },

    }).await?;

    println!("{:#?}", reply);

    println!("Reading audio");
    let bytes_per_second: usize = 2 * num_channels * sample_rate;
    let mut interval = time::interval(Duration::from_secs(1));

    for chunk in data.chunks(bytes_per_second).cycle() {
        let frame = Frame {
            channel: reply.index,
            offset_hi: 0,
            offset_low: 0,
            flags: 0,
            data: chunk.into(),
        };

        interval.next().await;
        client.send_frame(frame).await?;
    }

    Ok(())
}
