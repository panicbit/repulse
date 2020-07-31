use num_enum::{TryFromPrimitive, IntoPrimitive};
use anyhow::*;
use crate::{tag_struct, TagStruct};

#[derive(Debug)]
pub struct CommandHeader {
    pub command_kind: CommandKind,
    pub tag: u32,
}

impl tag_struct::Pop for CommandHeader {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        Ok(Self {
            command_kind: tag_struct.pop::<CommandKind>().context("Missing command kind field")?,
            tag: tag_struct.pop_u32().context("Missing tag field")?,
        })
    }
}

impl tag_struct::Put for CommandHeader {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.command_kind.into());
        tag_struct.put_u32(self.tag);
    }
}

pub struct Auth {
    pub protocol_version: u32,
    pub cookie: Vec<u8>,
}

impl tag_struct::Put for Auth {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.protocol_version);
        tag_struct.put_arbitrary(self.cookie);
    }
}

pub struct PlaySample {
    pub sink_index: u32,
    pub sink_name: Option<String>,
    pub volume: u32,
    pub sample_name: String,
}

impl tag_struct::Put for PlaySample {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.sink_index);
        tag_struct.put_string(self.sink_name);
        tag_struct.put_u32(self.volume);
        tag_struct.put_string(self.sample_name);
    }
}

#[derive(Debug)]
pub struct AuthReply {
    pub protocol_version: u32,
}

impl tag_struct::Pop for AuthReply {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        Ok(Self {
            protocol_version: tag_struct.pop_u32().context("Missing version field")?,
        })
    }   
}

#[derive(Debug, TryFromPrimitive, IntoPrimitive, Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum CommandKind {
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

impl CommandKind {
    pub fn is_error(&self) -> bool {
        *self == CommandKind::Error
    }

    pub fn is_reply(&self) -> bool {
        *self == CommandKind::Reply
    }
}

impl tag_struct::Put for CommandKind {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.into());
    }
}

impl tag_struct::Pop for CommandKind {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        let command = tag_struct.pop_u32().context("Missing command field")?;
        let command = CommandKind::try_from_primitive(command)
            .context("Failed to parse Command")?;

        Ok(command)
    }
}
