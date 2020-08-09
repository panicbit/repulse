use anyhow::*;
use futures::prelude::*;
use tokio::{fs, time::{self, Duration}};
use crate::{
    tag_struct::{SampleSpec, TagStruct, ChannelMap, ChannelVolume},
    command::{CreatePlaybackStream, SinkRef, CreatePlaybackStreamReply},
    sample::SampleFormat,
    channel::ChannelPosition,
};

pub use crate::{
    client::Client,
};

mod broker;
mod tag_struct;
mod client;
mod stream;
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

    let track_1 = tokio::spawn(play_raw_audio_forever(client.clone(), "🦀 Repulse - Track 1 🦀", "/tmp/audio1.raw"));
    // let track_2 = tokio::spawn(play_raw_audio_forever(client.clone(), "🦀 Repulse - Track 2 🦀", "/tmp/audio2.raw"));

    track_1.await??;
    // track_2.await??;

    Ok(())
}

async fn play_raw_audio_forever(client: Client, name: &'static str, file_name: &'static str) -> Result<()> {
    let data = fs::read(file_name).await?;
    let sample_rate: usize = 44_100;
    let num_channels: usize = 2;

    eprintln!("Creating playback stream");

    let sample_spec = SampleSpec {
        format: SampleFormat::S16LE,
        channels: num_channels as u8,
        rate: sample_rate as u32,
    };
    let channel_map = ChannelMap {
        positions: vec![
            ChannelPosition::FrontLeft,
            ChannelPosition::FrontRight,
        ],
    };
    let stream = client.create_playback_stream(name, sample_spec, channel_map).await?;

    println!("Reading audio");
    let bytes_per_second: usize = 2 * num_channels * sample_rate;
    let mut interval = time::interval(Duration::from_secs(1));

    for chunk in data.chunks(bytes_per_second).cycle() {
        interval.next().await;
        stream.write_slice(chunk).await?;
    }

    unreachable!()
}