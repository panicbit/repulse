use num_enum::{TryFromPrimitive, IntoPrimitive};
use anyhow::*;
use crate::{tag_struct, TagStruct, INVALID_INDEX};
use tag_struct::{ChannelMap, SampleSpec, ChannelVolume};

pub trait Command {
    const KIND: CommandKind;
}

pub type Tag = u32;

#[derive(Debug)]
pub struct CommandHeader {
    pub command_kind: CommandKind,
    pub tag: Tag,
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

impl Command for Auth {
    const KIND: CommandKind = CommandKind::Auth;
}

impl tag_struct::Put for Auth {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_u32(self.protocol_version);
        tag_struct.put_arbitrary(self.cookie);
    }
}

pub struct PlaySample {
    pub sink_ref: SinkRef,
    pub volume: u32,
    pub sample_name: String,
}

impl Command for PlaySample {
    const KIND: CommandKind = CommandKind::PlaySample;
}

impl tag_struct::Put for PlaySample {
    fn put(self, tag_struct: &mut TagStruct) {
        let (sink_index, sink_name) = match self.sink_ref {
            SinkRef::Index(index) => (index, None),
            SinkRef::Name(name) => (INVALID_INDEX, Some(name)),
        };

        tag_struct.put_u32(sink_index);
        tag_struct.put_string(sink_name);
        tag_struct.put_u32(self.volume);
        tag_struct.put_string(self.sample_name);
    }
}

#[derive(Debug)]
pub enum SinkRef {
    Index(u32),
    Name(String),
}

impl SinkRef {
    pub fn index(index: u32) -> Self {
        Self::Index(index)
    }

    pub fn name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
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

#[derive(Debug)]
pub struct CreatePlaybackStream {
    pub name: String,
    pub sample_spec: SampleSpec, //PA_TAG_SAMPLE_SPEC, &ss,
    pub channel_map: ChannelMap, //PA_TAG_CHANNEL_MAP, &map,
    pub sink_ref: SinkRef,
    //  pub sink_index: 0, // PA_TAG_U32, &sink_index,
    //  pub sink_name: String, // PA_TAG_STRING, &sink_name,
    pub max_length: u32,  // PA_TAG_U32, &attr.maxlength,
    pub corked: bool, // PA_TAG_BOOLEAN, &corked,
    pub t_length: u32, // PA_TAG_U32, &attr.tlength,
    pub prebuf: u32, // PA_TAG_U32, &attr.prebuf,
    pub min_req: u32, //PA_TAG_U32, &attr.minreq,
    pub sync_id: u32, //PA_TAG_U32, &syncid,
    pub volume: ChannelVolume, //PA_TAG_CVOLUME, &volume,
}

impl Command for CreatePlaybackStream {
    const KIND: CommandKind = CommandKind::CreatePlaybackStream;
}

impl tag_struct::Put for CreatePlaybackStream {
    fn put(self, tag_struct: &mut TagStruct) {
        tag_struct.put_string(self.name);
        tag_struct.put_sample_spec(self.sample_spec);
        tag_struct.put_channel_map(self.channel_map);
        
        let (sink_index, sink_name) = match self.sink_ref {
            SinkRef::Index(index) => (index, None),
            SinkRef::Name(name) => (INVALID_INDEX, Some(name)),
        };

        tag_struct.put_u32(sink_index);
        tag_struct.put_string(sink_name);

        tag_struct.put_u32(self.max_length);
        tag_struct.put_bool(self.corked);
        tag_struct.put_u32(self.t_length);
        tag_struct.put_u32(self.prebuf);
        tag_struct.put_u32(self.min_req);
        tag_struct.put_u32(self.sync_id);
        tag_struct.put_channel_volume(self.volume);
    }
}

#[derive(Debug)]
pub struct CreatePlaybackStreamReply {
    pub index: u32,
    pub sink_input: u32,
    pub missing: u32,
}

impl tag_struct::Pop for CreatePlaybackStreamReply {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        Ok(Self {
            index: tag_struct.pop_u32().context("Missing index field")?,
            sink_input: tag_struct.pop_u32().context("Missing sink_input field")?,
            missing: tag_struct.pop_u32().context("Missing missing field")?,
        })
    }
}

pub struct GetServerInfo;

impl Command for GetServerInfo {
    const KIND: CommandKind = CommandKind::GetServerInfo;
}

impl tag_struct::Put for GetServerInfo {
    fn put(self, _tag_struct: &mut TagStruct) {
    }
}

#[derive(Debug)]
pub struct ServerInfo {
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub user_name: Option<String>,
    pub host_name: Option<String>,
    pub sample_spec: SampleSpec,
    pub default_sink_name: Option<String>,
    pub default_source_name: Option<String>,
    pub instance_cookie: u32,
}

impl tag_struct::Pop for ServerInfo {
    fn pop(tag_struct: &mut TagStruct) -> Result<Self> {
        Ok(Self {
            server_name: tag_struct.pop_string()?,
            server_version: tag_struct.pop_string()?,
            user_name: tag_struct.pop_string()?,
            host_name: tag_struct.pop_string()?,
            sample_spec: tag_struct.pop_sample_spec()?,
            default_sink_name: tag_struct.pop_string()?,
            default_source_name: tag_struct.pop_string()?,
            instance_cookie: tag_struct.pop_u32()?,
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
