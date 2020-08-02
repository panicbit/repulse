use std::collections::VecDeque;
use std::convert::TryFrom;
use std::io::{Read, Write, Cursor, BufRead};
use anyhow::*;
use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use crate::{channel::{ChannelPosition, CHANNELS_MAX}, sample::SampleFormat};

#[derive(Debug, Default)]
pub struct TagStruct {
    values: VecDeque<Value>,
}

impl TagStruct {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let len = bytes.len() as u64;
        let mut bytes = Cursor::new(bytes);
        let mut values = VecDeque::new();

        while bytes.position() < len {
            let value = Value::read_from(&mut bytes)?;

            values.push_back(value);
        }

        Ok(Self { values })
    }

    pub fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        for value in &self.values {
            value.write_to(writer)?;
        }

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn to_vec(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();

        self.write_to(&mut data)?;

        Ok(data)
    }

    pub fn put<V: Put>(&mut self, value: V) {
        value.put(self);
    }

    pub fn pop<V: Pop>(&mut self) -> Result<V> {
        V::pop(self)
    }

    pub fn pop_value(&mut self) -> Result<Value> {
        self.values.pop_front().context("Missing value")
    }

    pub fn put_value(&mut self, value: Value) {
        self.values.push_back(value);
    }

    pub fn pop_bool(&mut self) -> Result<bool> {
        self.pop_value()?.into_bool()
    }

    pub fn put_bool(&mut self, value: bool) {
        self.put_value(Value::Bool(value));
    }

    pub fn pop_u8(&mut self) -> Result<u8> {
        self.pop_value()?.into_u8()
    }

    pub fn put_u8(&mut self, value: u8) {
        self.put_value(Value::U8(value));
    }

    pub fn pop_u32(&mut self) -> Result<u32> {
        self.pop_value()?.into_u32()
    }

    pub fn put_u32(&mut self, value: u32) {
        self.put_value(Value::U32(value));
    }

    pub fn pop_string(&mut self) -> Result<Option<String>> {
        self.pop_value()?.into_string()
    }

    pub fn put_string(&mut self, value: impl Into<Option<String>>) {
        self.put_value(Value::String(value.into()))
    }

    pub fn pop_arbitrary(&mut self) -> Result<Vec<u8>> {
        self.pop_value()?.into_arbitrary()
    }

    pub fn put_arbitrary(&mut self, value: Vec<u8>) {
        self.put_value(Value::Arbitrary(value));
    }

    pub fn put_channel_map(&mut self, value: ChannelMap) {
        self.put_value(Value::ChannelMap(value));
    }

    pub fn put_sample_spec(&mut self, value: SampleSpec) {
        self.put_value(Value::SampleSpec(value));
    }

    pub fn pop_sample_spec(&mut self) -> Result<SampleSpec> {
        self.pop_value()?.into_sample_spec()
    }

    pub fn put_channel_volume(&mut self, value: ChannelVolume) {
        self.put_value(Value::ChannelVolume(value));
    }
}

#[derive(Debug)]
pub enum Value {
    Bool(bool),
    U8(u8),
    U32(u32),
    String(Option<String>),
    Arbitrary(Vec<u8>),
    SampleSpec(SampleSpec),
    ChannelMap(ChannelMap),
    ChannelVolume(ChannelVolume),
}

impl Value {
    fn read_from<R: Read + BufRead>(reader: &mut R) -> Result<Self> {
        let tag = reader.read_u8()?;

        Ok(match tag {
            tag::BOOLEAN_TRUE => Value::Bool(true),
            tag::BOOLEAN_FALSE => Value::Bool(false),
            tag::U8 => Value::U8(reader.read_u8()?),
            tag::U32 => Value::U32(reader.read_u32::<BE>()?),
            tag::STRING_NULL => Value::String(None),
            tag::STRING => {
                let mut value = Vec::new();

                reader.read_until(b'\0', &mut value)?;

                let mut value = String::from_utf8(value)?;

                if value.as_bytes().last() != Some(&b'\0') {
                    bail!("String did not end with NULL");
                }

                value.pop();

                Value::String(Some(value))
            }
            tag::ARBITRARY => {
                let len = reader.read_u32::<BE>()?;
                let len = usize::try_from(len)
                    .context("Arbitrary value len exceeds pointer width")?;
                let mut value = vec![0; len];
                reader.read_exact(&mut value)?;

                Value::Arbitrary(value)
            },
            tag::SAMPLE_SPEC => {
                let value = SampleSpec {
                    format: {
                        let format = reader.read_u8()?;
                        SampleFormat::try_from(format)?
                    },
                    channels:  reader.read_u8()?,
                    rate: reader.read_u32::<BE>()?,
                };

                Value::SampleSpec(value)
            }
            _ => bail!("Unimplemented tag '{}'", tag as char),
        })
    }

    pub fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        match self {
            Self::Bool(value) => match value {
                true => writer.write_u8(tag::BOOLEAN_TRUE)?,
                false => writer.write_u8(tag::BOOLEAN_FALSE)?,
            },
            Self::U8(value) => {
                writer.write_u8(tag::U8)?;
                writer.write_u8(*value)?;
            },
            Self::U32(value) => {
                writer.write_u8(tag::U32)?;
                writer.write_u32::<BE>(*value)?;
            },
            Self::Arbitrary(value) => {
                writer.write_u8(tag::ARBITRARY)?;
                let len = u32::try_from(value.len())
                    .context("Arbitrary value len exceeds 32 bits")?;
                writer.write_u32::<BE>(len)?;
                writer.write_all(value)?;
            },
            Self::String(value) => {
                match value {
                    Some(value) => {
                        writer.write_u8(tag::STRING)?;
                        let value = truncate_to_before_first_null(value);
                        writer.write_all(value.as_bytes())?;
                        writer.write_u8(b'\0')?;
                    },
                    None => {
                        writer.write_u8(tag::STRING_NULL)?;
                        return Ok(());
                    }
                };
            },
            Value::SampleSpec(spec) => {
                writer.write_u8(tag::SAMPLE_SPEC)?;
                writer.write_u8(spec.format.into())?;
                writer.write_u8(spec.channels)?;
                writer.write_u32::<BE>(spec.rate)?;
            },
            Value::ChannelMap(map) => {
                writer.write_u8(tag::CHANNEL_MAP)?;

                let num_channels = CHANNELS_MAX.min(map.positions.len());
                writer.write_u8(num_channels as u8)?;

                for &position in map.positions.iter().take(num_channels) {
                    writer.write_u8(position.into())?;
                }
            },
            Value::ChannelVolume(volume) => {
                writer.write_u8(tag::CVOLUME)?;

                let num_channels = CHANNELS_MAX.min(volume.volumes.len());
                writer.write_u8(num_channels as u8)?;

                for &volume in volume.volumes.iter().take(num_channels) {
                    writer.write_u32::<BE>(volume)?;
                }
            },
        }

        Ok(())
    }

    fn into_bool(self) -> Result<bool> {
        match self {
            Self::Bool(value) => Ok(value),
            _ => bail!("Expected bool value"),
        }
    }

    fn into_u8(self) -> Result<u8> {
        match self {
            Self::U8(value) => Ok(value),
            _ => bail!("Expected u8 value"),
        }
    }

    fn into_u32(self) -> Result<u32> {
        match self {
            Self::U32(value) => Ok(value),
            _ => bail!("Expected u32 value"),
        }
    }

    fn into_string(self) -> Result<Option<String>> {
        match self {
            Self::String(value) => Ok(value),
            _ => bail!("Expected string value")
        }
    }

    fn into_arbitrary(self) -> Result<Vec<u8>> {
        match self {
            Self::Arbitrary(value) => Ok(value),
            _ => bail!("Expected arbitrary value")
        }
    }

    fn into_sample_spec(self) -> Result<SampleSpec> {
        match self {
            Self::SampleSpec(value) => Ok(value),
            _ => bail!("Expected sample spec value")
        }
    }
}


#[derive(Debug)]
pub struct SampleSpec {
    pub format: SampleFormat,
    pub channels: u8,
    pub rate: u32,
}

#[derive(Debug)]
pub struct ChannelMap {
    /// Channel positions
    pub positions: Vec<ChannelPosition>, // Use SmallVec?
}

#[derive(Debug)]
pub struct ChannelVolume {
    /// Volume per channel
    pub volumes: Vec<u32>, // Use SmallVec?
}

mod tag {
    pub const INVALID: u8 = 0;
    pub const STRING: u8 = b't';
    pub const STRING_NULL: u8 = b'N';
    pub const U32: u8 = b'L';
    pub const U8: u8 = b'B';
    pub const U64: u8 = b'R';
    pub const S64: u8 = b'r';
    pub const SAMPLE_SPEC: u8 = b'a';
    pub const ARBITRARY: u8 = b'x';
    pub const BOOLEAN_TRUE: u8 = b'1';
    pub const BOOLEAN_FALSE: u8 = b'0';
    pub const BOOLEAN: u8 = BOOLEAN_TRUE;
    pub const TIMEVAL: u8 = b'T';
    pub const USEC: u8 = b'U'; // 64bit unsigned
    pub const CHANNEL_MAP: u8 = b'm';
    pub const CVOLUME: u8 = b'v';
    pub const PROPLIST: u8 = b'P';
    pub const VOLUME: u8 = b'V';
    pub const FORMAT_INFO: u8 = b'f';
}

pub trait Pop: Sized {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self>;
}

pub trait Put {
    fn put(self, tag_struct: &mut TagStruct);
}

fn truncate_to_before_first_null(s: &str) -> &str {
    s.split('\0').next().unwrap()
}
