use num_enum::{TryFromPrimitive, IntoPrimitive};

#[derive(TryFromPrimitive, IntoPrimitive, Debug, Copy, Clone)]
#[repr(u8)]
pub enum SampleFormat {
    /// Unsigned 8 Bit PCM
    U8,
    /// 8 Bit a-Law
    ALAW,
    /// 8 Bit mu-Law
    ULAW,
    /// Signed 16 Bit PCM, little endian (PC)
    S16LE,
    /// Signed 16 Bit PCM, big endian
    S16BE,
    /// 32 Bit IEEE floating point, little endian (PC), range -1.0 to 1.0
    FLOAT32LE,
    /// 32 Bit IEEE floating point, big endian, range -1.0 to 1.0
    FLOAT32BE,
    /// Signed 32 Bit PCM, little endian (PC)
    S32LE,
    /// Signed 32 Bit PCM, big endian
    S32BE,
    /// Signed 24 Bit PCM packed, little endian (PC). \since 0.9.15
    S24LE,
    /// Signed 24 Bit PCM packed, big endian. \since 0.9.15
    S24BE,
    /// Signed 24 Bit PCM in LSB of 32 Bit words, little endian (PC). \since 0.9.15
    S24_32LE,
    /// Signed 24 Bit PCM in LSB of 32 Bit words, big endian. \since 0.9.15
    S24_32BE,
    /// Upper limit of valid sample types
    MAX,
    /// An invalid value
    INVALID = u8::MAX,
}
