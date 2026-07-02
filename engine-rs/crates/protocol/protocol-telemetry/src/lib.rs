//! Structured telemetry border shapes.
//!
//! # Lane
//!
//! `contract-steward` — owns observational telemetry types mirrored to generated
//! TypeScript. Telemetry is never authority: it carries counters, gauges, spans,
//! and trace messages for tools/debugging and must not be used as a domain event
//! bus.

#![forbid(unsafe_code)]

use serde::Serialize;

/// Stable telemetry source strings in declaration order.
pub const TELEMETRY_SOURCES: &[&str] = &["runtime", "policy", "renderer", "devtools", "replay"];

/// Stable telemetry severity strings in declaration order.
pub const TELEMETRY_LEVELS: &[&str] = &["debug", "info", "warning", "error"];

/// Stable telemetry metric-kind strings in declaration order.
pub const TELEMETRY_METRIC_KINDS: &[&str] = &["counter", "gauge", "durationMs"];

/// Component that produced an observational telemetry event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TelemetrySource {
    Runtime,
    Policy,
    Renderer,
    Devtools,
    Replay,
}

impl TelemetrySource {
    pub fn as_str(self) -> &'static str {
        match self {
            TelemetrySource::Runtime => "runtime",
            TelemetrySource::Policy => "policy",
            TelemetrySource::Renderer => "renderer",
            TelemetrySource::Devtools => "devtools",
            TelemetrySource::Replay => "replay",
        }
    }
}

/// Severity of an observational telemetry event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TelemetryLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl TelemetryLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            TelemetryLevel::Debug => "debug",
            TelemetryLevel::Info => "info",
            TelemetryLevel::Warning => "warning",
            TelemetryLevel::Error => "error",
        }
    }
}

/// Metric value category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TelemetryMetricKind {
    Counter,
    Gauge,
    DurationMs,
}

impl TelemetryMetricKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TelemetryMetricKind::Counter => "counter",
            TelemetryMetricKind::Gauge => "gauge",
            TelemetryMetricKind::DurationMs => "durationMs",
        }
    }
}

/// One numeric telemetry sample.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryMetric {
    pub name: String,
    pub kind: TelemetryMetricKind,
    pub value: f64,
    pub unit: Option<String>,
}

/// One observational telemetry event.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TelemetryEvent {
    Metric {
        source: TelemetrySource,
        level: TelemetryLevel,
        sequence: u64,
        metric: TelemetryMetric,
    },
    Trace {
        source: TelemetrySource,
        level: TelemetryLevel,
        sequence: u64,
        span: String,
        message: String,
    },
}

impl TelemetryEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            TelemetryEvent::Metric { .. } => "metric",
            TelemetryEvent::Trace { .. } => "trace",
        }
    }
}

/// A batch of telemetry events emitted for one observation point.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEnvelope {
    pub protocol_version: u32,
    pub emitted_at_tick: u64,
    pub events: Vec<TelemetryEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_stable_vocabularies() {
        assert_eq!(TelemetrySource::Runtime.as_str(), TELEMETRY_SOURCES[0]);
        assert_eq!(TelemetryLevel::Warning.as_str(), TELEMETRY_LEVELS[2]);
        assert_eq!(
            TelemetryMetricKind::DurationMs.as_str(),
            TELEMETRY_METRIC_KINDS[2]
        );
        let event = TelemetryEvent::Trace {
            source: TelemetrySource::Runtime,
            level: TelemetryLevel::Info,
            sequence: 1,
            span: "boot".to_string(),
            message: "ready".to_string(),
        };
        assert_eq!(event.kind(), "trace");
    }
}
