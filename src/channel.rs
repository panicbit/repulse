use num_enum::{TryFromPrimitive, IntoPrimitive};

pub const CHANNELS_MAX: usize = 32;

#[derive(TryFromPrimitive, IntoPrimitive, Debug, Copy, Clone)]
#[repr(u8)]
pub enum ChannelPosition {
    Invalid = u8::MAX,
    Mono = 0,

    /// Apple, Dolby call this 'Left'
    FrontLeft,
    /// Apple, Dolby call this 'Right'
    FrontRight,
    /// Apple, Dolby call this 'Center'
    FrontCenter,

    /// Microsoft calls this 'Back Center', Apple calls this 'Center Surround', Dolby calls this 'Surround Rear Center'
    RearCenter,
    /// Microsoft calls this 'Back Left', Apple calls this 'Left Surround' (!), Dolby calls this 'Surround Rear Left' 
    RearLeft,
    /// Microsoft calls this 'Back Right', Apple calls this 'Right Surround' (!), Dolby calls this 'Surround Rear Right' 
    RearRight,

    /// Microsoft calls this 'Low Frequency', Apple calls this 'LFEScreen'
    Lfe,

    /// Apple, Dolby call this 'Left Center'
    FrontLeftOfCenter,
    /// Apple, Dolby call this 'Right Center
    FrontRightOfCenter,

    /// Apple calls this 'Left Surround Direct', Dolby calls this 'Surround Left' (!)
    SideLeft,
    /// Apple calls this 'Right Surround Direct', Dolby calls this 'Surround Right' (!)
    SideRight,

    Aux0,
    Aux1,
    Aux2,
    Aux3,
    Aux4,
    Aux5,
    Aux6,
    Aux7,
    Aux8,
    Aux9,
    Aux10,
    Aux11,
    Aux12,
    Aux13,
    Aux14,
    Aux15,
    Aux16,
    Aux17,
    Aux18,
    Aux19,
    Aux20,
    Aux21,
    Aux22,
    Aux23,
    Aux24,
    Aux25,
    Aux26,
    Aux27,
    Aux28,
    Aux29,
    Aux30,
    Aux31,

    /// Apple calls this 'Top Center Surround'
    TopCenter,

    /// Apple calls this 'Vertical Height Left'
    TopFrontLeft,
    /// Apple calls this 'Vertical Height Right'
    TopFrontRight,
    /// Apple calls this 'Vertical Height Center'
    TopFrontCenter,

    /// Microsoft and Apple call this 'Top Back Left'
    TopRearLeft,
    /// Microsoft and Apple call this 'Top Back Right'
    TopRearRight,
    /// Microsoft and Apple call this 'Top Back Center'
    TopRearCenter,
}
