use anyhow::*;
use tokio::prelude::*;
use crate::TagStruct;
use tokio_util::codec::{FramedRead, Decoder};
use bytes::{Buf, BytesMut};
use std::{mem::size_of, convert::TryFrom};

pub const COMMAND_CHANNEL: u32 = u32::max_value();

#[derive(Debug)]
pub struct Frame {
    pub channel: u32,
    pub offset_hi: u32,
    pub offset_low: u32,
    pub flags: u32,
    pub data: BytesMut,
}

impl Frame {
    pub fn command(packet: &TagStruct) -> Result<Frame> {
        Ok(Self {
            channel: COMMAND_CHANNEL,
            offset_hi: 0,
            offset_low: 0,
            flags: 0,
            data: packet.to_bytes()?,
        })
    }

    pub fn is_command_frame(&self) -> bool {
        self.channel == COMMAND_CHANNEL
    }

    pub async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        writer.write_u32(self.data.len() as u32).await?;
        writer.write_u32(self.channel).await?;
        writer.write_u32(self.offset_hi).await?;
        writer.write_u32(self.offset_low).await?;
        writer.write_u32(self.flags).await?;
        writer.write_all(&self.data).await?;
        Ok(())
    }

    pub fn stream<R: AsyncRead>(reader: R) -> FramedRead<R, FrameDecoder> {
        FramedRead::new(reader, FrameDecoder::default())
    }
}

#[derive(Default)]
pub struct FrameDecoder {
    data_len: Option<usize>,
}

impl Decoder for FrameDecoder {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Frame>> {
        let data_len = match self.data_len {
            Some(data_len) => data_len,
            None => {
                if src.len() < 4 {
                    src.reserve(5 * size_of::<u32>() - src.len());
                    return Ok(None);
                }

                let data_len = src.get_u32();
                let data_len = usize::try_from(data_len)
                    .context("frame length exceeds platform pointer size")?;

                self.data_len = Some(data_len);
                data_len
            }
        };

        let remaining_len = data_len + 4 * size_of::<u32>();

        if src.len() < remaining_len {
            src.reserve(remaining_len - src.len());
            return Ok(None);
        }

        self.data_len = None;

        Ok(Some(dbg!(Frame {
            channel: src.get_u32(),
            offset_hi: src.get_u32(),
            offset_low: src.get_u32(),
            flags: src.get_u32(),
            data: src.split_to(data_len),
        })))
    }
}
