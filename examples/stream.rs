use anyhow::*;
use lewton::{
    header::HeaderSet,
    inside_ogg::async_api::{HeadersReader, OggStreamReader, read_headers}
};
use futures::{TryStreamExt, StreamExt};
use ogg::reading::async_api::PacketReader;
use repulse::{sample::SampleFormat, tag_struct::{ChannelMap, SampleSpec}};
use std::mem::size_of;
use tokio::{io, time};
use time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    loop {
        if let Err(err) = play_rainwave_stream().await {
            eprintln!("{:?}", err);
            time::delay_for(Duration::from_secs(1)).await;
        }
    }
}

async fn play_rainwave_stream() -> Result<()> {
    let response = reqwest::get("http://allstream.rainwave.cc:8000/all.ogg").await?
    .error_for_status()?;

    println!("{:#?}", response);

    let stream = response.bytes_stream().map_err(|err| {
        io::Error::new(io::ErrorKind::Other, err)
    });
    let stream = io::stream_reader(stream);

    let mut packet_reader = PacketReader::new(stream);

    let pulseaudio = repulse::Client::connect().await?;

    let headers = read_headers(&mut packet_reader).await?;
    let info = &headers.0;

    let sample_spec = dbg!(SampleSpec {
        format: SampleFormat::S16LE,
        channels: info.audio_channels,
        rate: info.audio_sample_rate,
    });
    let bytes_per_second = sample_spec.channels as usize * size_of::<i16>() * sample_spec.rate as usize;
    let channel_map = match sample_spec.channels {
        // 1 => ChannelMap::mono(),
        2 => ChannelMap::stereo(),
        _ => bail!("Only stereo streams are supported"),
    };

    let playback_stream = pulseaudio.create_playback_stream("Rainwave - All", sample_spec, channel_map).await?;

    let mut stream_reader = OggStreamReader::from_pck_rdr(packet_reader, headers);

    let mut data = Vec::new();
    let mut interval = time::interval(Duration::from_secs(1));

    while let Some(channels) = stream_reader.next().await {
        let mut channels = channels?;
        let channel1 = channels.pop().unwrap();
        let channel2 = channels.pop().unwrap();
        let interleaved_samples = channel1.into_iter().zip(channel2);

        for (sample1, sample2) in interleaved_samples {
            data.extend_from_slice(&sample1.to_le_bytes());
            data.extend_from_slice(&sample2.to_le_bytes());
        }

        if data.len() >= bytes_per_second {
            interval.next().await;
            playback_stream.write_slice(&data).await?;
            data.clear();
        }
    }

    interval.next().await;

    Ok(())
}
