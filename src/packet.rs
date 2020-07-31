use num_enum::{TryFromPrimitive, IntoPrimitive};
use anyhow::*;
use crate::{tag_struct, TagStruct};

#[derive(Debug)]
pub struct PacketHeader {
    pub command: Command,
    pub tag: u32,
}

impl tag_struct::Pop for PacketHeader {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        Ok(Self {
            command: tag_struct.pop::<Command>().context("Missing command field")?,
            tag: tag_struct.pop_u32().context("Missing tag field")?,
        })
    }
}

impl tag_struct::Put for PacketHeader {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.command.into());
        tag_struct.put_u32(self.tag);
    }
}

#[derive(Debug, TryFromPrimitive, IntoPrimitive, Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum Command {
    /* Generic commands */
    Error,
    Timeout, /* pseudo command */
    Reply,

    /* CLIENT->SERVER */
    CreatePlaybackStream,        /* Payload changed in v9, v12 (0.9.0, 0.9.8) */
    DeletePlaybackStream,
    CreateRecordStream,          /* Payload changed in v9, v12 (0.9.0, 0.9.8) */
    DeleteRecordStream,
    Exit,
    Auth,
    SetClientName,
    LookupSink,
    LookupSource,
    DrainPlaybackStream,
    Stat,
    GetPlaybackLatency,
    CreateUploadStream,
    DeleteUploadStream,
    FinishUploadStream,
    PlaySample,
    RemoveSample,

    GetServerInfo,
    GetSinkInfo,
    GetSinkInfoList,
    GetSourceInfo,
    GetSourceInfoList,
    GetModuleInfo,
    GetModuleInfoList,
    GetClientInfo,
    GetClientInfoList,
    GetSinkInputInfo,          /* Payload changed in v11 (0.9.7) */
    GetSinkInputInfoList,     /* Payload changed in v11 (0.9.7) */
    GetSourceOutputInfo,
    GetSourceOutputInfoList,
    GetSampleInfo,
    GetSampleInfoList,
    Subscribe,

    SetSinkVolume,
    SetSinkInputVolume,
    SetSourceVolume,

    SetSinkMute,
    SetSourceMute,

    CorkPlaybackStream,
    FlushPlaybackStream,
    TriggerPlaybackStream,

    SetDefaultSink,
    SetDefaultSource,

    SetPlaybackStreamName,
    SetRecordStreamName,

    KillClient,
    KillSinkInput,
    KillSourceOutput,

    LoadModule,
    UnloadModule,

    /* Obsolete */
    #[allow(warnings)] AddAutoload__Obsolete,
    #[allow(warnings)] RemoveAutoload__Obsolete,
    #[allow(warnings)] GetAutoloadInfo__Obsolete,
    #[allow(warnings)] GetAutoloadInfoList__Obsolete,

    GetRecordLatency,
    CorkRecordStream,
    FlushRecordStream,
    PrebufPlaybackStream,

    /* SERVER->CLIENT */
    Request,
    Overflow,
    Underflow,
    PlaybackStreamKilled,
    RecordStreamKilled,
    SubscribeEvent,

    /* A few more client->server commands */

    /* Supported since protocol v10 (0.9.5) */
    MoveSinkInput,
    MoveSourceOutput,

    /* Supported since protocol v11 (0.9.7) */
    SetSinkInputMute,

    SuspendSink,
    SuspendSource,

    /* Supported since protocol v12 (0.9.8) */
    SetPlaybackStreamBufferAttr,
    SetRecordStreamBufferAttr,

    UpdatePlaybackStreamSampleRate,
    UpdateRecordStreamSampleRate,

    /* SERVER->CLIENT */
    PlaybackStreamSuspended,
    RecordStreamSuspended,
    PlaybackStreamMoved,
    RecordStreamMoved,

    /* Supported since protocol v13 (0.9.11) */
    UpdateRecordStreamProplist,
    UpdatePlaybackStreamProplist,
    UpdateClientProplist,
    RemoveRecordStreamProplist,
    RemovePlaybackStreamProplist,
    RemoveClientProplist,

    /* SERVER->CLIENT */
    Started,

    /* Supported since protocol v14 (0.9.12) */
    Extension,

    /* Supported since protocol v15 (0.9.15) */
    GetCardInfo,
    GetCardInfoList,
    SetCardProfile,

    ClientEvent,
    PlaybackStreamEvent,
    RecordStreamEvent,

    /* SERVER->CLIENT */
    PlaybackBufferAttrChanged,
    RecordBufferAttrChanged,

    /* Supported since protocol v16 (0.9.16) */
    SetSinkPort,
    SetSourcePort,

    /* Supported since protocol v22 (1.0) */
    SetSourceOutputVolume,
    SetSourceOutputMute,

    /* Supported since protocol v27 (3.0) */
    SetPortLatencyOffset,

    /* Supported since protocol v30 (6.0) */
    /* BOTH DIRECTIONS */
    EnableSrbChannel,
    DisableSrbChannel,

    /* Supported since protocol v31 (9.0)
     * BOTH DIRECTIONS */
    RegisterMemFdShmId,
}

impl tag_struct::Put for Command {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.into());
    }
}

impl tag_struct::Pop for Command {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        let command = tag_struct.pop_u32().context("Missing command field")?;
        let command = Command::try_from_primitive(command)
            .context("Failed to parse Command")?;

        Ok(command)
    }
}
