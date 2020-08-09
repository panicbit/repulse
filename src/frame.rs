use anyhow::*;
use tokio::prelude::*;
use tokio_util::codec;
use bytes::{Buf, BytesMut, BufMut};
use std::{mem::size_of, convert::TryFrom};
use crate::tag_struct::TagStruct;

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

    pub fn stream<R: AsyncRead>(reader: R) -> codec::FramedRead<R, Decoder> {
        codec::FramedRead::new(reader, Decoder::default())
    }

    pub fn sink<W: AsyncWrite>(writer: W) -> codec::FramedWrite<W, Encoder> {
        codec::FramedWrite::new(writer, Encoder::default())
    }
}

#[derive(Default)]
pub struct Decoder {
    data_len: Option<usize>,
}

impl codec::Decoder for Decoder {
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

#[derive(Default)]
pub struct Encoder(());

impl codec::Encoder<Frame> for Encoder {
    type Error = Error;

    fn encode(&mut self, frame: Frame, dst: &mut BytesMut) -> Result<()> {
        let len = u32::try_from(frame.data.len())
            .context("Frame data size does not fit into 32 bits")?;

        dst.put_u32(len);
        dst.put_u32(frame.channel);
        dst.put_u32(frame.offset_hi);
        dst.put_u32(frame.offset_low);
        dst.put_u32(frame.flags);
        dst.put_slice(&frame.data);

        Ok(())
    }
}
