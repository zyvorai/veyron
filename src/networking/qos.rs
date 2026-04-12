use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Traffic class
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficClass {
    RealTime,
    Critical,
    Priority,
    BestEffort,
}

/// QoS policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSPolicy {
    pub id: String,
    pub name: String,
    pub class: TrafficClass,
    pub min_bandwidth_mbps: Option<u32>,
    pub max_bandwidth_mbps: Option<u32>,
    pub guaranteed_bandwidth_mbps: Option<u32>,
    pub burst_size_kb: Option<u32>,
    pub priority: u32,
    pub dscp: Option<u8>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl QoSPolicy {
    pub fn new(name: impl Into<String>, class: TrafficClass) -> Self {
        let name_str = name.into();
        let id = format!(
            "qos-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            class,
            min_bandwidth_mbps: None,
            max_bandwidth_mbps: None,
            guaranteed_bandwidth_mbps: None,
            burst_size_kb: None,
            priority: 100,
            dscp: None,
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_min_bandwidth(mut self, mbps: u32) -> Self {
        self.min_bandwidth_mbps = Some(mbps);
        self
    }

    pub fn with_max_bandwidth(mut self, mbps: u32) -> Self {
        self.max_bandwidth_mbps = Some(mbps);
        self
    }

    pub fn with_guaranteed_bandwidth(mut self, mbps: u32) -> Self {
        self.guaranteed_bandwidth_mbps = Some(mbps);
        self
    }

    pub fn with_burst_size(mut self, kb: u32) -> Self {
        self.burst_size_kb = Some(kb);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_dscp(mut self, dscp: u8) -> Self {
        self.dscp = Some(dscp.min(63));
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn has_bandwidth_limits(&self) -> bool {
        self.min_bandwidth_mbps.is_some() || self.max_bandwidth_mbps.is_some()
    }
}

/// Traffic shaping rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficShapingRule {
    pub id: String,
    pub name: String,
    pub rate_limit_mbps: u32,
    pub burst_limit_mbps: Option<u32>,
    pub latency_ms: Option<u32>,
    pub packet_loss_percent: Option<f64>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl TrafficShapingRule {
    pub fn new(name: impl Into<String>, rate_limit_mbps: u32) -> Self {
        let name_str = name.into();
        let id = format!(
            "shape-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            rate_limit_mbps,
            burst_limit_mbps: None,
            latency_ms: None,
            packet_loss_percent: None,
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_burst_limit(mut self, mbps: u32) -> Self {
        self.burst_limit_mbps = Some(mbps);
        self
    }

    pub fn with_latency(mut self, ms: u32) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    pub fn with_packet_loss(mut self, percent: f64) -> Self {
        self.packet_loss_percent = Some(percent.clamp(0.0, 100.0));
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

/// Bandwidth reservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthReservation {
    pub id: String,
    pub resource_id: String,
    pub reserved_mbps: u32,
    pub guaranteed: bool,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl BandwidthReservation {
    pub fn new(resource_id: impl Into<String>, reserved_mbps: u32) -> Self {
        let resource_id_str = resource_id.into();
        let id = format!("bw-{}-{}", resource_id_str, Utc::now().timestamp_micros());

        Self {
            id,
            resource_id: resource_id_str,
            reserved_mbps,
            guaranteed: false,
            start_time: Utc::now(),
            end_time: None,
            created_at: Utc::now(),
        }
    }

    pub fn guarantee(mut self) -> Self {
        self.guaranteed = true;
        self
    }

    pub fn with_duration(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = start;
        self.end_time = Some(end);
        self
    }

    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        if let Some(end) = self.end_time {
            now >= self.start_time && now <= end
        } else {
            now >= self.start_time
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(end) = self.end_time {
            Utc::now() > end
        } else {
            false
        }
    }
}

/// QoS manager
pub struct QoSManager {
    policies: HashMap<String, QoSPolicy>,
    shaping_rules: HashMap<String, TrafficShapingRule>,
    reservations: HashMap<String, BandwidthReservation>,
}

impl QoSManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            shaping_rules: HashMap::new(),
            reservations: HashMap::new(),
        }
    }

    pub fn add_policy(&mut self, policy: QoSPolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&QoSPolicy> {
        self.policies.get(id)
    }

    pub fn get_policy_mut(&mut self, id: &str) -> Option<&mut QoSPolicy> {
        self.policies.get_mut(id)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn add_shaping_rule(&mut self, rule: TrafficShapingRule) -> String {
        let id = rule.id.clone();
        self.shaping_rules.insert(id.clone(), rule);
        id
    }

    pub fn get_shaping_rule(&self, id: &str) -> Option<&TrafficShapingRule> {
        self.shaping_rules.get(id)
    }

    pub fn shaping_rule_count(&self) -> usize {
        self.shaping_rules.len()
    }

    pub fn add_reservation(&mut self, reservation: BandwidthReservation) -> String {
        let id = reservation.id.clone();
        self.reservations.insert(id.clone(), reservation);
        id
    }

    pub fn get_reservation(&self, id: &str) -> Option<&BandwidthReservation> {
        self.reservations.get(id)
    }

    pub fn reservation_count(&self) -> usize {
        self.reservations.len()
    }

    pub fn enabled_policies(&self) -> Vec<&QoSPolicy> {
        self.policies.values().filter(|p| p.enabled).collect()
    }

    pub fn policies_by_class(&self, class: &TrafficClass) -> Vec<&QoSPolicy> {
        self.policies
            .values()
            .filter(|p| &p.class == class)
            .collect()
    }

    pub fn enabled_shaping_rules(&self) -> Vec<&TrafficShapingRule> {
        self.shaping_rules.values().filter(|r| r.enabled).collect()
    }

    pub fn active_reservations(&self) -> Vec<&BandwidthReservation> {
        self.reservations
            .values()
            .filter(|r| r.is_active())
            .collect()
    }

    pub fn guaranteed_reservations(&self) -> Vec<&BandwidthReservation> {
        self.reservations
            .values()
            .filter(|r| r.guaranteed)
            .collect()
    }

    pub fn total_reserved_bandwidth(&self) -> u32 {
        self.reservations
            .values()
            .filter(|r| r.is_active())
            .map(|r| r.reserved_mbps)
            .sum()
    }
}

impl Default for QoSManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_policy() {
        let policy = QoSPolicy::new("high-priority", TrafficClass::Priority);

        assert_eq!(policy.name, "high-priority");
        assert_eq!(policy.class, TrafficClass::Priority);
        assert!(policy.enabled);
    }

    #[test]
    fn test_policy_with_min_bandwidth() {
        let policy = QoSPolicy::new("test", TrafficClass::Critical).with_min_bandwidth(100);

        assert_eq!(policy.min_bandwidth_mbps, Some(100));
    }

    #[test]
    fn test_policy_with_max_bandwidth() {
        let policy = QoSPolicy::new("test", TrafficClass::BestEffort).with_max_bandwidth(500);

        assert_eq!(policy.max_bandwidth_mbps, Some(500));
    }

    #[test]
    fn test_policy_with_guaranteed_bandwidth() {
        let policy = QoSPolicy::new("test", TrafficClass::RealTime).with_guaranteed_bandwidth(200);

        assert_eq!(policy.guaranteed_bandwidth_mbps, Some(200));
    }

    #[test]
    fn test_policy_with_burst_size() {
        let policy = QoSPolicy::new("test", TrafficClass::Priority).with_burst_size(1024);

        assert_eq!(policy.burst_size_kb, Some(1024));
    }

    #[test]
    fn test_policy_with_priority() {
        let policy = QoSPolicy::new("test", TrafficClass::Critical).with_priority(10);

        assert_eq!(policy.priority, 10);
    }

    #[test]
    fn test_policy_with_dscp() {
        let policy = QoSPolicy::new("test", TrafficClass::RealTime).with_dscp(46);

        assert_eq!(policy.dscp, Some(46));
    }

    #[test]
    fn test_policy_dscp_clamping() {
        let policy = QoSPolicy::new("test", TrafficClass::RealTime).with_dscp(100);

        assert_eq!(policy.dscp, Some(63));
    }

    #[test]
    fn test_policy_disable_enable() {
        let mut policy = QoSPolicy::new("test", TrafficClass::Priority);

        assert!(policy.enabled);

        policy.disable();
        assert!(!policy.enabled);

        policy.enable();
        assert!(policy.enabled);
    }

    #[test]
    fn test_policy_has_bandwidth_limits() {
        let policy1 = QoSPolicy::new("test1", TrafficClass::Priority);
        assert!(!policy1.has_bandwidth_limits());

        let policy2 = QoSPolicy::new("test2", TrafficClass::Priority).with_min_bandwidth(100);
        assert!(policy2.has_bandwidth_limits());

        let policy3 = QoSPolicy::new("test3", TrafficClass::Priority).with_max_bandwidth(500);
        assert!(policy3.has_bandwidth_limits());
    }

    #[test]
    fn test_traffic_shaping_rule() {
        let rule = TrafficShapingRule::new("limit-1gbps", 1000);

        assert_eq!(rule.name, "limit-1gbps");
        assert_eq!(rule.rate_limit_mbps, 1000);
        assert!(rule.enabled);
    }

    #[test]
    fn test_shaping_rule_with_burst_limit() {
        let rule = TrafficShapingRule::new("test", 100).with_burst_limit(200);

        assert_eq!(rule.burst_limit_mbps, Some(200));
    }

    #[test]
    fn test_shaping_rule_with_latency() {
        let rule = TrafficShapingRule::new("test", 100).with_latency(50);

        assert_eq!(rule.latency_ms, Some(50));
    }

    #[test]
    fn test_shaping_rule_with_packet_loss() {
        let rule = TrafficShapingRule::new("test", 100).with_packet_loss(0.5);

        assert_eq!(rule.packet_loss_percent, Some(0.5));
    }

    #[test]
    fn test_shaping_rule_packet_loss_clamping() {
        let rule1 = TrafficShapingRule::new("test1", 100).with_packet_loss(150.0);
        assert_eq!(rule1.packet_loss_percent, Some(100.0));

        let rule2 = TrafficShapingRule::new("test2", 100).with_packet_loss(-10.0);
        assert_eq!(rule2.packet_loss_percent, Some(0.0));
    }

    #[test]
    fn test_shaping_rule_disable_enable() {
        let mut rule = TrafficShapingRule::new("test", 100);

        assert!(rule.enabled);

        rule.disable();
        assert!(!rule.enabled);

        rule.enable();
        assert!(rule.enabled);
    }

    #[test]
    fn test_bandwidth_reservation() {
        let reservation = BandwidthReservation::new("vm-123", 500);

        assert_eq!(reservation.resource_id, "vm-123");
        assert_eq!(reservation.reserved_mbps, 500);
        assert!(!reservation.guaranteed);
    }

    #[test]
    fn test_reservation_guarantee() {
        let reservation = BandwidthReservation::new("vm-1", 200).guarantee();

        assert!(reservation.guaranteed);
    }

    #[test]
    fn test_reservation_with_duration() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::hours(24);

        let reservation = BandwidthReservation::new("vm-1", 300).with_duration(start, end);

        assert_eq!(reservation.start_time, start);
        assert_eq!(reservation.end_time, Some(end));
    }

    #[test]
    fn test_reservation_is_active() {
        let start = Utc::now() - chrono::TimeDelta::hours(1);
        let end = Utc::now() + chrono::TimeDelta::hours(1);

        let reservation1 = BandwidthReservation::new("vm-1", 100).with_duration(start, end);
        assert!(reservation1.is_active());

        let past_start = Utc::now() - chrono::TimeDelta::hours(2);
        let past_end = Utc::now() - chrono::TimeDelta::hours(1);
        let reservation2 =
            BandwidthReservation::new("vm-2", 100).with_duration(past_start, past_end);
        assert!(!reservation2.is_active());
    }

    #[test]
    fn test_reservation_is_expired() {
        let past_start = Utc::now() - chrono::TimeDelta::hours(2);
        let past_end = Utc::now() - chrono::TimeDelta::hours(1);

        let reservation1 =
            BandwidthReservation::new("vm-1", 100).with_duration(past_start, past_end);
        assert!(reservation1.is_expired());

        let reservation2 = BandwidthReservation::new("vm-2", 100);
        assert!(!reservation2.is_expired());
    }

    #[test]
    fn test_qos_manager() {
        let mut manager = QoSManager::new();

        let policy = QoSPolicy::new("test", TrafficClass::Priority);
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_add_shaping_rule() {
        let mut manager = QoSManager::new();

        let rule = TrafficShapingRule::new("test", 1000);
        let id = manager.add_shaping_rule(rule);

        assert_eq!(manager.shaping_rule_count(), 1);
        assert!(manager.get_shaping_rule(&id).is_some());
    }

    #[test]
    fn test_manager_add_reservation() {
        let mut manager = QoSManager::new();

        let reservation = BandwidthReservation::new("vm-1", 500);
        let id = manager.add_reservation(reservation);

        assert_eq!(manager.reservation_count(), 1);
        assert!(manager.get_reservation(&id).is_some());
    }

    #[test]
    fn test_manager_enabled_policies() {
        let mut manager = QoSManager::new();

        let policy1 = QoSPolicy::new("p1", TrafficClass::Priority);
        let mut policy2 = QoSPolicy::new("p2", TrafficClass::BestEffort);
        policy2.disable();

        manager.add_policy(policy1);
        manager.add_policy(policy2);

        let enabled = manager.enabled_policies();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_policies_by_class() {
        let mut manager = QoSManager::new();

        manager.add_policy(QoSPolicy::new("p1", TrafficClass::Priority));
        manager.add_policy(QoSPolicy::new("p2", TrafficClass::Critical));
        manager.add_policy(QoSPolicy::new("p3", TrafficClass::Priority));

        let priority = manager.policies_by_class(&TrafficClass::Priority);
        assert_eq!(priority.len(), 2);
    }

    #[test]
    fn test_manager_enabled_shaping_rules() {
        let mut manager = QoSManager::new();

        let rule1 = TrafficShapingRule::new("r1", 100);
        let mut rule2 = TrafficShapingRule::new("r2", 200);
        rule2.disable();

        manager.add_shaping_rule(rule1);
        manager.add_shaping_rule(rule2);

        let enabled = manager.enabled_shaping_rules();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_active_reservations() {
        let mut manager = QoSManager::new();

        let start = Utc::now() - chrono::TimeDelta::hours(1);
        let end = Utc::now() + chrono::TimeDelta::hours(1);

        manager.add_reservation(BandwidthReservation::new("vm-1", 100).with_duration(start, end));

        let past_start = Utc::now() - chrono::TimeDelta::hours(2);
        let past_end = Utc::now() - chrono::TimeDelta::hours(1);
        manager.add_reservation(
            BandwidthReservation::new("vm-2", 200).with_duration(past_start, past_end),
        );

        let active = manager.active_reservations();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_guaranteed_reservations() {
        let mut manager = QoSManager::new();

        manager.add_reservation(BandwidthReservation::new("vm-1", 100).guarantee());
        manager.add_reservation(BandwidthReservation::new("vm-2", 200));

        let guaranteed = manager.guaranteed_reservations();
        assert_eq!(guaranteed.len(), 1);
    }

    #[test]
    fn test_manager_total_reserved_bandwidth() {
        let mut manager = QoSManager::new();

        let start = Utc::now() - chrono::TimeDelta::hours(1);
        let end = Utc::now() + chrono::TimeDelta::hours(1);

        manager.add_reservation(BandwidthReservation::new("vm-1", 100).with_duration(start, end));
        manager.add_reservation(BandwidthReservation::new("vm-2", 200).with_duration(start, end));

        assert_eq!(manager.total_reserved_bandwidth(), 300);
    }

    #[test]
    fn test_traffic_class_equality() {
        assert_eq!(TrafficClass::Priority, TrafficClass::Priority);
        assert_ne!(TrafficClass::Priority, TrafficClass::Critical);
    }
}
