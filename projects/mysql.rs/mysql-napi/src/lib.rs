//! Node bindings: thin wrappers over `database-protocol-mysql` framing and the
//! upstream `database.server::yyds::mysql` gateway handles.
//!
//! No bin, no global state and no CLI: the TypeScript host owns the command
//! line. The server handles in [`server`] compose the upstream gateway; no
//! protocol bytes live here.

mod server;

use database::harness::{
    Baseline, BaselineIdentity, DifferenceObservation, Digest, ReportEnvelope, Stats, check_baseline, render_json, summarize,
};
use napi::{Error, Result, Status, bindgen_prelude::Buffer};
use napi_derive::napi;

fn map_error(error: database_protocol_mysql::PacketError) -> Error {
    Error::new(Status::InvalidArg, format!("{error:?}"))
}

/// One decoded classic packet.
#[napi(object)]
pub struct DecodedPacket {
    /// Sequence byte from the 4-byte header.
    pub sequence: u32,
    /// Payload bytes the header announced.
    pub payload: Buffer,
}

/// Largest payload one classic packet may carry.
#[napi]
pub fn max_packet_payload() -> u32 {
    database_protocol_mysql::MAX_PACKET_PAYLOAD as u32
}

/// Protocol version announced in the initial handshake.
#[napi]
pub fn protocol_version() -> u8 {
    database_protocol_mysql::PROTOCOL_VERSION
}

/// Frames one payload with the classic 4-byte header.
#[napi]
pub fn encode_packet(sequence: u32, payload: Buffer) -> Result<Buffer> {
    let sequence = u8::try_from(sequence).map_err(|_| Error::new(Status::InvalidArg, "sequence must be 0..=255"))?;
    database_protocol_mysql::encode_packet(sequence, payload.as_ref()).map(Buffer::from).map_err(map_error)
}

/// Decodes the first complete packet, leaving the rest of the input untouched.
#[napi]
pub fn decode_packet(input: Buffer) -> Result<Option<DecodedPacket>> {
    match database_protocol_mysql::decode_packet(input.as_ref(), database_protocol_mysql::MAX_PACKET_PAYLOAD)
        .map_err(map_error)?
    {
        Some(packet) => {
            Ok(Some(DecodedPacket { sequence: u32::from(packet.sequence), payload: Buffer::from(packet.payload.to_vec()) }))
        }
        None => Ok(None),
    }
}

fn harness_error(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

/// 对给定字节取 SHA-256，返回小写十六进制。
#[napi]
pub fn harness_digest(bytes: Buffer) -> String {
    Digest::of(bytes.as_ref()).to_hex()
}

/// 按已知差异台账判定一次语料运行。
#[napi]
pub fn harness_check_baseline(
    baseline_json: String,
    current_json: String,
    observed_json: String,
) -> Result<String> {
    let baseline: Baseline = serde_json::from_str(&baseline_json).map_err(harness_error)?;
    let current: BaselineIdentity = serde_json::from_str(&current_json).map_err(harness_error)?;
    let observed: Vec<DifferenceObservation> = serde_json::from_str(&observed_json).map_err(harness_error)?;
    let verdict = check_baseline(&baseline, &current, &observed);
    serde_json::to_string(&verdict).map_err(harness_error)
}

/// 将报告信封渲染为稳定 JSON 文本。
#[napi]
pub fn harness_render_report(report_json: String) -> Result<String> {
    let report: ReportEnvelope = serde_json::from_str(&report_json).map_err(harness_error)?;
    render_json(&report).map_err(harness_error)
}

/// 基准统计的 JS 形态。字段名由本 DTO 负责 camelCase，核心 `Stats` 说 Rust。
#[napi(object)]
pub struct BenchStatsJson {
    pub median_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub rounds: u32,
}

impl From<Stats> for BenchStatsJson {
    fn from(stats: Stats) -> Self {
        Self { median_ms: stats.median_ms, p95_ms: stats.p95_ms, min_ms: stats.min_ms, max_ms: stats.max_ms, rounds: stats.rounds }
    }
}

/// 对一组毫秒样本求 median/p95/极值（中间两值 median、nearest-rank p95，与各仓既有实现逐位一致）。
#[napi]
pub fn harness_bench_summary(samples_ms: Vec<f64>) -> BenchStatsJson {
    summarize(&samples_ms).into()
}
