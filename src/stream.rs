use anyhow::*;
use crate::{frame::Frame, Client};
use bytes::BytesMut;

#[derive(Clone)]
pub struct PlaybackStream {
    channel: u32,
    client: Client,
}

impl PlaybackStream {
    pub(crate) fn new(client: &Client, channel: u32) -> Self {
        Self {
            channel,
            client: client.clone(),
        }
    }

    /// This is currently slightly more efficient than `write_slice`.
    pub async fn write_bytes(&self, data: BytesMut) -> Result<()> {
        let frame = Frame {
            channel: self.channel,
            offset_hi: 0,
            offset_low: 0,
            flags: 0,
            data: data.into(),
        };

        self.client.send_frame(frame).await?;

        Ok(())
    }

    // FIXME: takes &[u8] because BytesMut doesn't implement Into<Vec<u8>>
    pub async fn write_slice(&self, data: &[u8]) -> Result<()> {
        self.write_bytes(data.into()).await
    }
}