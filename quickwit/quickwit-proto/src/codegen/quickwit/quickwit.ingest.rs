#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DocBatchV2 {
    #[prost(bytes = "bytes", tag = "1")]
    pub doc_buffer: ::prost::bytes::Bytes,
    #[prost(uint32, repeated, tag = "2")]
    pub doc_lengths: ::prost::alloc::vec::Vec<u32>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MRecordBatch {
    /// Buffer of encoded and then concatenated mrecords.
    #[prost(bytes = "bytes", tag = "1")]
    pub mrecord_buffer: ::prost::bytes::Bytes,
    /// Lengths of the mrecords in the buffer.
    #[prost(uint32, repeated, tag = "2")]
    pub mrecord_lengths: ::prost::alloc::vec::Vec<u32>,
}
#[derive(Eq)]
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Shard {
    /// Immutable fields
    #[prost(message, optional, tag = "1")]
    pub index_uid: ::core::option::Option<crate::types::IndexUid>,
    #[prost(string, tag = "2")]
    pub source_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "3")]
    pub shard_id: ::core::option::Option<crate::types::ShardId>,
    /// The node ID of the ingester to which all the write requests for this shard should be sent to.
    #[prost(string, tag = "4")]
    pub leader_id: ::prost::alloc::string::String,
    /// The node ID of the ingester holding a copy of the data.
    #[prost(string, optional, tag = "5")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follower_id: ::core::option::Option<::prost::alloc::string::String>,
    /// Mutable fields
    #[prost(enumeration = "ShardState", tag = "8")]
    pub shard_state: i32,
    /// Position up to which indexers have indexed and published the records stored in the shard.
    /// It is updated asynchronously in a best effort manner by the indexers and indicates the position up to which the log can be safely truncated.
    #[prost(message, optional, tag = "9")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish_position_inclusive: ::core::option::Option<crate::types::Position>,
    /// A publish token that ensures only one indexer works on a given shard at a time.
    /// For instance, if an indexer goes rogue, eventually the control plane will detect it and assign the shard to another indexer, which will override the publish token.
    #[prost(string, optional, tag = "10")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish_token: ::core::option::Option<::prost::alloc::string::String>,
}
/// A group of shards belonging to the same index and source.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ShardIds {
    #[prost(message, optional, tag = "1")]
    pub index_uid: ::core::option::Option<crate::types::IndexUid>,
    #[prost(string, tag = "2")]
    pub source_id: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub shard_ids: ::prost::alloc::vec::Vec<crate::types::ShardId>,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CommitTypeV2 {
    Unspecified = 0,
    Auto = 1,
    Wait = 2,
    Force = 3,
}
impl CommitTypeV2 {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            CommitTypeV2::Unspecified => "COMMIT_TYPE_V2_UNSPECIFIED",
            CommitTypeV2::Auto => "COMMIT_TYPE_V2_AUTO",
            CommitTypeV2::Wait => "COMMIT_TYPE_V2_WAIT",
            CommitTypeV2::Force => "COMMIT_TYPE_V2_FORCE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "COMMIT_TYPE_V2_UNSPECIFIED" => Some(Self::Unspecified),
            "COMMIT_TYPE_V2_AUTO" => Some(Self::Auto),
            "COMMIT_TYPE_V2_WAIT" => Some(Self::Wait),
            "COMMIT_TYPE_V2_FORCE" => Some(Self::Force),
            _ => None,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ShardState {
    Unspecified = 0,
    /// The shard is open and accepts write requests.
    Open = 1,
    /// The ingester hosting the shard is unavailable.
    Unavailable = 2,
    /// The shard is closed and cannot be written to.
    /// It can be safely deleted if the publish position is superior or equal to `~eof`.
    Closed = 3,
}
impl ShardState {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ShardState::Unspecified => "SHARD_STATE_UNSPECIFIED",
            ShardState::Open => "SHARD_STATE_OPEN",
            ShardState::Unavailable => "SHARD_STATE_UNAVAILABLE",
            ShardState::Closed => "SHARD_STATE_CLOSED",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SHARD_STATE_UNSPECIFIED" => Some(Self::Unspecified),
            "SHARD_STATE_OPEN" => Some(Self::Open),
            "SHARD_STATE_UNAVAILABLE" => Some(Self::Unavailable),
            "SHARD_STATE_CLOSED" => Some(Self::Closed),
            _ => None,
        }
    }
}
