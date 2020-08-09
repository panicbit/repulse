use anyhow::*;
use futures::prelude::*;
use tokio::{fs, time::{self, Duration}};
use repulse::{
    Client,
    tag_struct::{SampleSpec, ChannelMap},
};

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
    let bit_depth = 2;

    eprintln!("Creating playback stream");

    let sample_spec = SampleSpec::pcm_signed_16bit_little_endian_stereo_44100hz();
    let channel_map = ChannelMap::stereo();
    let stream = client.create_playback_stream(name, sample_spec, channel_map).await?;

    println!("Reading audio");
    let bytes_per_second: usize = bit_depth * num_channels * sample_rate;
    let mut interval = time::interval(Duration::from_secs(1));

    for chunk in data.chunks(bytes_per_second).cycle() {
        interval.next().await;
        stream.write_slice(chunk).await?;
    }

    unreachable!()
}
