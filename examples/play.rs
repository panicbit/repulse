use anyhow::*;
use futures::prelude::*;
use tokio::{time::{self, Duration}};
use repulse::{
    Client,
    tag_struct::{SampleSpec, ChannelMap}, sample::SampleFormat,
};
use cauldron::audio::{ChannelLayout, AudioSegment};
use std::mem::size_of;

#[tokio::main]
async fn main() -> Result<()> {
    let filename = std::env::args().nth(1)
        .context("first argument must be a file to play")?;
    
    let mut audio_segment = AudioSegment::read(&filename)
        .context("Failed to open file")?;
    
    let info = audio_segment.info();
    let sample_spec = SampleSpec {
        format: SampleFormat::S16LE,
        channels: info.channels.count() as u8,
        rate: info.sample_rate,
    };
    let channel_map = match info.channel_layout {
        ChannelLayout::Mono => ChannelMap::mono(),
        ChannelLayout::Stereo => ChannelMap::stereo(),
        _ => bail!("Only mono and stereo audio is supported right now"),
    };

    let samples = audio_segment.samples::<i16>()
        .context("Failed to get samples")?;

    let mut audio = Vec::new();

    for sample in samples {
        let sample = sample.context("Failed to decode sample")?;

        audio.extend_from_slice(&sample.to_le_bytes());
    }

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
