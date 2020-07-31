use anyhow::*;
use tokio::prelude::*;
use crate::TagStruct;

pub const COMMAND_CHANNEL: u32 = u32::max_value();

#[derive(Debug)]
pub struct Frame {
    pub channel: u32,
    pub offset_hi: u32,
    pub offset_low: u32,
    pub flags: u32,
    pub data: Vec<u8>,
}

impl Frame {
    pub fn command(packet: &TagStruct) -> Result<Frame> {
        Ok(Self {
            channel: COMMAND_CHANNEL,
            offset_hi: 0,
            offset_low: 0,
            flags: 0,
            data: packet.to_vec()?,
        })
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

    pub async fn read_from<R>(reader: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let size = reader.read_u32().await?;
        let channel = reader.read_u32().await?;
        let offset_hi = reader.read_u32().await?;
        let offset_low = reader.read_u32().await?;
        let flags = reader.read_u32().await?;
        let mut data = vec![0; size as usize];
        reader.read_exact(&mut data).await?;

        Ok(Self {
            channel,
            offset_hi,
            offset_low,
            flags,
            data,
        })
    }
}
