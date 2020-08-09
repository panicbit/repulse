use anyhow::*;
use futures::prelude::*;
use tokio::{time::{self, Duration}};
use repulse::{
    Client,
    tag_struct::{SampleSpec, ChannelMap}, sample::SampleFormat,
};
use std::mem::size_of;

#[tokio::main]
async fn main() -> Result<()> {
    let filename = std::env::args().nth(1)
        .context("first argument must be a file to play")?;
    
    let mut reader = audrey::read::open(&filename)
        .context("Failed to open file")?;
    
    let info = reader.description();
    let sample_spec = SampleSpec {
        format: SampleFormat::S16LE,
        channels: info.channel_count() as u8,
        rate: info.sample_rate(),
    };
    let channel_map = match sample_spec.channels {
        1 => ChannelMap::mono(),
        2 => ChannelMap::stereo(),
        _ => bail!("Only mono and stereo audio is supported right now"),
    };

    let samples = reader.samples::<i16>();

    let mut audio = Vec::new();

    for sample in samples {
        let sample = sample.context("Failed to decode sample")?;

        audio.extend_from_slice(&sample.to_le_bytes());
    }

    println!("PCM length: {}", audio.len());

    let client = Client::connect().await
        .context("Failed to create client")?;
    
    let bit_depth = size_of::<i16>();
    let bytes_per_second: usize = bit_depth * sample_spec.channels as usize * sample_spec.rate as usize;

    let stream = client.create_playback_stream(filename, sample_spec, channel_map).await?;

    let mut interval = time::interval(Duration::from_secs(1));

    for chunk in audio.chunks(bytes_per_second).cycle() {
        interval.next().await;
        stream.write_slice(chunk).await?;
    }

    interval.next().await;

    Ok(())
}
