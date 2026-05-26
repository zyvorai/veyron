// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Traffic Analysis - Network traffic analysis and flow monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Traffic flow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficFlow {
    pub source_ip: String,
    pub dest_ip: String,
    pub source_port: u16,
    pub dest_port: u16,
    pub protocol: Protocol,
    pub bytes: u64,
    pub packets: u64,
    pub start_time: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl TrafficFlow {
    pub fn new(
        source_ip: impl Into<String>,
        dest_ip: impl Into<String>,
        source_port: u16,
        dest_port: u16,
        protocol: Protocol,
    ) -> Self {
        Self {
            source_ip: source_ip.into(),
            dest_ip: dest_ip.into(),
            source_port,
            dest_port,
            protocol,
            bytes: 0,
            packets: 0,
            start_time: Utc::now(),
            last_seen: Utc::now(),
        }
    }

    /// Get flow duration in seconds
    pub fn duration_secs(&self) -> i64 {
        self.last_seen
            .signed_duration_since(self.start_time)
            .num_seconds()
    }

    /// Calculate average bytes per second
    pub fn bytes_per_sec(&self) -> f64 {
        let duration = self.duration_secs();
        if duration <= 0 {
            0.0
        } else {
            self.bytes as f64 / duration as f64
        }
    }

    /// Generate flow key for identification
    pub fn flow_key(&self) -> String {
        format!(
            "{}:{}-{}:{}-{}",
            self.source_ip,
            self.source_port,
            self.dest_ip,
            self.dest_port,
            self.protocol.as_str()
        )
    }
}

/// Network protocol
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
    Other(u8),
}

impl Protocol {
    pub fn as_str(&self) -> &str {
        match self {
            Protocol::TCP => "TCP",
            Protocol::UDP => "UDP",
            Protocol::ICMP => "ICMP",
            Protocol::Other(_) => "OTHER",
        }
    }

    pub fn from_num(num: u8) -> Self {
        match num {
            6 => Protocol::TCP,
            17 => Protocol::UDP,
            1 => Protocol::ICMP,
            n => Protocol::Other(n),
        }
    }
}

/// Traffic analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSummary {
    pub interface_name: String,
    pub total_flows: usize,
    pub active_flows: usize,
    pub total_bytes: u64,
    pub total_packets: u64,
    pub protocol_breakdown: HashMap<String, ProtocolStats>,
    pub top_talkers: Vec<TopTalker>,
    pub analysis_time: DateTime<Utc>,
}

impl TrafficSummary {
    pub fn new(interface_name: impl Into<String>) -> Self {
        Self {
            interface_name: interface_name.into(),
            total_flows: 0,
            active_flows: 0,
            total_bytes: 0,
            total_packets: 0,
            protocol_breakdown: HashMap::new(),
            top_talkers: Vec::new(),
            analysis_time: Utc::now(),
        }
    }

    /// Get protocol percentage
    pub fn protocol_percent(&self, protocol: &str) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }

        if let Some(stats) = self.protocol_breakdown.get(protocol) {
            (stats.bytes as f64 / self.total_bytes as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Protocol statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    pub protocol: String,
    pub flows: usize,
    pub bytes: u64,
    pub packets: u64,
}

impl ProtocolStats {
    pub fn new(protocol: impl Into<String>) -> Self {
        Self {
            protocol: protocol.into(),
            flows: 0,
            bytes: 0,
            packets: 0,
        }
    }
}

/// Top talker (host with most traffic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTalker {
    pub ip_address: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub total_bytes: u64,
    pub flow_count: usize,
}

impl TopTalker {
    pub fn new(ip_address: impl Into<String>) -> Self {
        Self {
            ip_address: ip_address.into(),
            bytes_sent: 0,
            bytes_received: 0,
            total_bytes: 0,
            flow_count: 0,
        }
    }

    /// Calculate traffic percentage
    pub fn traffic_percent(&self, total_traffic: u64) -> f64 {
        if total_traffic == 0 {
            0.0
        } else {
            (self.total_bytes as f64 / total_traffic as f64) * 100.0
        }
    }
}

/// Traffic analyzer for flow analysis
pub struct TrafficAnalyzer {
    interface_name: String,
    flows: HashMap<String, TrafficFlow>,
    flow_timeout_secs: i64,
    max_flows: usize,
}

impl TrafficAnalyzer {
    pub fn new(interface_name: impl Into<String>) -> Self {
        Self {
            interface_name: interface_name.into(),
            flows: HashMap::new(),
            flow_timeout_secs: 300, // 5 minutes
            max_flows: 100_000,
        }
    }

    pub fn with_timeout(mut self, timeout_secs: i64) -> Self {
        self.flow_timeout_secs = timeout_secs;
        self
    }

    /// Add or update a flow
    pub fn add_flow(&mut self, flow: TrafficFlow) {
        let key = flow.flow_key();

        if let Some(existing) = self.flows.get_mut(&key) {
            existing.bytes += flow.bytes;
            existing.packets += flow.packets;
            existing.last_seen = flow.last_seen;
        } else {
            // Evict expired flows if at capacity before inserting
            if self.flows.len() >= self.max_flows {
                self.cleanup_expired_flows();
            }
            // If still at capacity after cleanup, remove oldest flow
            if self.flows.len() >= self.max_flows {
                if let Some(oldest_key) = self
                    .flows
                    .iter()
                    .min_by_key(|(_, f)| f.last_seen)
                    .map(|(k, _)| k.clone())
                {
                    self.flows.remove(&oldest_key);
                }
            }
            self.flows.insert(key, flow);
        }
    }

    /// Remove expired flows
    pub fn cleanup_expired_flows(&mut self) {
        let now = Utc::now();
        self.flows.retain(|_, flow| {
            now.signed_duration_since(flow.last_seen).num_seconds() < self.flow_timeout_secs
        });
    }

    /// Get active flow count
    pub fn active_flow_count(&self) -> usize {
        let now = Utc::now();
        self.flows
            .values()
            .filter(|f| {
                now.signed_duration_since(f.last_seen).num_seconds() < self.flow_timeout_secs
            })
            .count()
    }

    /// Generate traffic summary
    pub fn generate_summary(&self, top_n: usize) -> TrafficSummary {
        let mut summary = TrafficSummary::new(&self.interface_name);
        summary.total_flows = self.flows.len();
        summary.active_flows = self.active_flow_count();

        // Calculate totals and protocol breakdown
        let mut protocol_map: HashMap<String, ProtocolStats> = HashMap::new();

        for flow in self.flows.values() {
            summary.total_bytes = summary.total_bytes.saturating_add(flow.bytes);
            summary.total_packets = summary.total_packets.saturating_add(flow.packets);

            let proto_name = flow.protocol.as_str().to_string();
            let stats = protocol_map
                .entry(proto_name.clone())
                .or_insert_with(|| ProtocolStats::new(proto_name));

            stats.flows += 1;
            stats.bytes = stats.bytes.saturating_add(flow.bytes);
            stats.packets = stats.packets.saturating_add(flow.packets);
        }

        summary.protocol_breakdown = protocol_map;

        // Calculate top talkers
        let mut talker_map: HashMap<String, TopTalker> = HashMap::new();

        for flow in self.flows.values() {
            // Source
            let src = talker_map
                .entry(flow.source_ip.clone())
                .or_insert_with(|| TopTalker::new(&flow.source_ip));
            src.bytes_sent = src.bytes_sent.saturating_add(flow.bytes);
            src.total_bytes = src.total_bytes.saturating_add(flow.bytes);
            src.flow_count += 1;

            // Destination
            let dst = talker_map
                .entry(flow.dest_ip.clone())
                .or_insert_with(|| TopTalker::new(&flow.dest_ip));
            dst.bytes_received = dst.bytes_received.saturating_add(flow.bytes);
            dst.total_bytes = dst.total_bytes.saturating_add(flow.bytes);
        }

        // Sort and take top N
        let mut talkers: Vec<TopTalker> = talker_map.into_values().collect();
        talkers.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
        talkers.truncate(top_n);
        summary.top_talkers = talkers;

        summary
    }

    /// Get total flow count
    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_flow_creation() {
        let flow = TrafficFlow::new("10.0.0.1", "10.0.0.2", 8080, 80, Protocol::TCP);
        assert_eq!(flow.source_ip, "10.0.0.1");
        assert_eq!(flow.dest_ip, "10.0.0.2");
        assert_eq!(flow.source_port, 8080);
        assert_eq!(flow.dest_port, 80);
        assert_eq!(flow.protocol, Protocol::TCP);
    }

    #[test]
    fn test_flow_key() {
        let flow = TrafficFlow::new("10.0.0.1", "10.0.0.2", 8080, 80, Protocol::TCP);
        assert_eq!(flow.flow_key(), "10.0.0.1:8080-10.0.0.2:80-TCP");
    }

    #[test]
    fn test_protocol() {
        assert_eq!(Protocol::TCP.as_str(), "TCP");
        assert_eq!(Protocol::UDP.as_str(), "UDP");
        assert_eq!(Protocol::ICMP.as_str(), "ICMP");

        assert_eq!(Protocol::from_num(6), Protocol::TCP);
        assert_eq!(Protocol::from_num(17), Protocol::UDP);
        assert_eq!(Protocol::from_num(1), Protocol::ICMP);
    }

    #[test]
    fn test_traffic_analyzer() {
        let mut analyzer = TrafficAnalyzer::new("eth0");

        let mut flow1 = TrafficFlow::new("10.0.0.1", "10.0.0.2", 8080, 80, Protocol::TCP);
        flow1.bytes = 1000;
        flow1.packets = 10;

        analyzer.add_flow(flow1);
        assert_eq!(analyzer.flow_count(), 1);

        let summary = analyzer.generate_summary(5);
        assert_eq!(summary.total_flows, 1);
        assert_eq!(summary.total_bytes, 1000);
        assert_eq!(summary.total_packets, 10);
    }

    #[test]
    fn test_protocol_breakdown() {
        let mut analyzer = TrafficAnalyzer::new("eth0");

        let mut tcp_flow = TrafficFlow::new("10.0.0.1", "10.0.0.2", 8080, 80, Protocol::TCP);
        tcp_flow.bytes = 1000;

        let mut udp_flow = TrafficFlow::new("10.0.0.1", "10.0.0.3", 53, 53, Protocol::UDP);
        udp_flow.bytes = 500;

        analyzer.add_flow(tcp_flow);
        analyzer.add_flow(udp_flow);

        let summary = analyzer.generate_summary(5);
        assert_eq!(summary.protocol_breakdown.len(), 2);
        assert!(summary.protocol_breakdown.contains_key("TCP"));
        assert!(summary.protocol_breakdown.contains_key("UDP"));

        assert_eq!(summary.protocol_breakdown.get("TCP").unwrap().bytes, 1000);
        assert_eq!(summary.protocol_breakdown.get("UDP").unwrap().bytes, 500);
    }

    #[test]
    fn test_top_talkers() {
        let mut analyzer = TrafficAnalyzer::new("eth0");

        let mut flow1 = TrafficFlow::new("10.0.0.1", "10.0.0.2", 8080, 80, Protocol::TCP);
        flow1.bytes = 1000;

        let mut flow2 = TrafficFlow::new("10.0.0.1", "10.0.0.3", 8081, 80, Protocol::TCP);
        flow2.bytes = 2000;

        analyzer.add_flow(flow1);
        analyzer.add_flow(flow2);

        let summary = analyzer.generate_summary(5);
        assert!(!summary.top_talkers.is_empty());

        // 10.0.0.1 should be the top talker (sent 3000 bytes)
        let top = &summary.top_talkers[0];
        assert_eq!(top.ip_address, "10.0.0.1");
        assert_eq!(top.bytes_sent, 3000);
    }

    #[test]
    fn test_traffic_percent() {
        let mut talker = TopTalker::new("10.0.0.1");
        talker.total_bytes = 500;

        assert_eq!(talker.traffic_percent(1000), 50.0);
        assert_eq!(talker.traffic_percent(0), 0.0);
    }

    #[test]
    fn test_protocol_percent() {
        let mut summary = TrafficSummary::new("eth0");
        summary.total_bytes = 1000;

        let mut tcp_stats = ProtocolStats::new("TCP");
        tcp_stats.bytes = 600;
        summary
            .protocol_breakdown
            .insert("TCP".to_string(), tcp_stats);

        assert_eq!(summary.protocol_percent("TCP"), 60.0);
        assert_eq!(summary.protocol_percent("UDP"), 0.0);
    }
}
