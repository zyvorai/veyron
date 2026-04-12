use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DNS record type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    NS,
    PTR,
    SRV,
}

/// DNS record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSRecord {
    pub id: String,
    pub name: String,
    pub record_type: RecordType,
    pub value: String,
    pub ttl: u32,
    pub priority: Option<u16>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DNSRecord {
    /// Validate DNS record value based on record type.
    fn validate_dns_value(record_type: &RecordType, value: &str) -> anyhow::Result<()> {
        match record_type {
            RecordType::A => {
                value.parse::<std::net::Ipv4Addr>().map_err(|_| {
                    anyhow::anyhow!("A record value must be a valid IPv4 address: {}", value)
                })?;
            }
            RecordType::AAAA => {
                value.parse::<std::net::Ipv6Addr>().map_err(|_| {
                    anyhow::anyhow!("AAAA record value must be a valid IPv6 address: {}", value)
                })?;
            }
            _ => {} // CNAME, MX, TXT, SRV, PTR, NS values have varied formats
        }
        // Validate no control characters in value
        if value.chars().any(|c| c.is_control()) {
            anyhow::bail!("DNS record value must not contain control characters");
        }
        Ok(())
    }

    pub fn new(
        name: impl Into<String>,
        record_type: RecordType,
        value: impl Into<String>,
        ttl: u32,
    ) -> anyhow::Result<Self> {
        let name_str = name.into();
        let value_str = value.into();
        Self::validate_dns_value(&record_type, &value_str)?;
        let id = format!(
            "dns-{}-{}",
            name_str.replace('.', "-"),
            Utc::now().timestamp_micros()
        );

        Ok(Self {
            id,
            name: name_str,
            record_type,
            value: value_str,
            ttl,
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn with_priority(mut self, priority: u16) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn update_value(&mut self, value: impl Into<String>) -> anyhow::Result<()> {
        let new_value = value.into();
        Self::validate_dns_value(&self.record_type, &new_value)?;
        self.value = new_value;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn is_long_ttl(&self) -> bool {
        self.ttl >= 3600
    }
}

/// DNS zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSZone {
    pub id: String,
    pub name: String,
    pub primary_ns: String,
    pub admin_email: String,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum_ttl: u32,
    pub records: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl DNSZone {
    pub fn new(
        name: impl Into<String>,
        primary_ns: impl Into<String>,
        admin_email: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "zone-{}-{}",
            name_str.replace('.', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            primary_ns: primary_ns.into(),
            admin_email: admin_email.into(),
            serial: 1,
            refresh: 3600,
            retry: 600,
            expire: 86400,
            minimum_ttl: 300,
            records: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_record(&mut self, record_id: impl Into<String>) {
        self.records.push(record_id.into());
        self.increment_serial();
    }

    pub fn increment_serial(&mut self) {
        self.serial = self.serial.saturating_add(1);
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }
}

/// DNS manager
pub struct DNSManager {
    zones: HashMap<String, DNSZone>,
    records: HashMap<String, DNSRecord>,
}

impl DNSManager {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            records: HashMap::new(),
        }
    }

    pub fn add_zone(&mut self, zone: DNSZone) -> String {
        let id = zone.id.clone();
        self.zones.insert(id.clone(), zone);
        id
    }

    pub fn get_zone(&self, id: &str) -> Option<&DNSZone> {
        self.zones.get(id)
    }

    pub fn get_zone_mut(&mut self, id: &str) -> Option<&mut DNSZone> {
        self.zones.get_mut(id)
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn add_record(&mut self, record: DNSRecord) -> String {
        let id = record.id.clone();
        self.records.insert(id.clone(), record);
        id
    }

    pub fn get_record(&self, id: &str) -> Option<&DNSRecord> {
        self.records.get(id)
    }

    pub fn get_record_mut(&mut self, id: &str) -> Option<&mut DNSRecord> {
        self.records.get_mut(id)
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn records_by_type(&self, record_type: &RecordType) -> Vec<&DNSRecord> {
        self.records
            .values()
            .filter(|r| &r.record_type == record_type)
            .collect()
    }

    pub fn find_record_by_name(&self, name: &str) -> Vec<&DNSRecord> {
        self.records.values().filter(|r| r.name == name).collect()
    }
}

impl Default for DNSManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_record() {
        let record = DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 300).unwrap();

        assert_eq!(record.name, "example.com");
        assert_eq!(record.record_type, RecordType::A);
        assert_eq!(record.value, "192.168.1.1");
        assert_eq!(record.ttl, 300);
    }

    #[test]
    fn test_dns_record_invalid_a_record() {
        assert!(DNSRecord::new("example.com", RecordType::A, "not-an-ip", 300).is_err());
    }

    #[test]
    fn test_dns_record_invalid_aaaa_record() {
        assert!(DNSRecord::new("example.com", RecordType::AAAA, "not-an-ipv6", 300).is_err());
    }

    #[test]
    fn test_dns_record_control_chars() {
        assert!(DNSRecord::new("example.com", RecordType::TXT, "hello\x00world", 300).is_err());
    }

    #[test]
    fn test_record_with_priority() {
        let record = DNSRecord::new("example.com", RecordType::MX, "mail.example.com", 300)
            .unwrap()
            .with_priority(10);

        assert_eq!(record.priority, Some(10));
    }

    #[test]
    fn test_record_update_value() {
        let mut record = DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 300).unwrap();

        record.update_value("192.168.1.2").unwrap();
        assert_eq!(record.value, "192.168.1.2");
    }

    #[test]
    fn test_record_is_long_ttl() {
        let record1 =
            DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 3600).unwrap();
        assert!(record1.is_long_ttl());

        let record2 = DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 300).unwrap();
        assert!(!record2.is_long_ttl());
    }

    #[test]
    fn test_dns_zone() {
        let zone = DNSZone::new("example.com", "ns1.example.com", "admin@example.com");

        assert_eq!(zone.name, "example.com");
        assert_eq!(zone.primary_ns, "ns1.example.com");
        assert_eq!(zone.admin_email, "admin@example.com");
        assert_eq!(zone.serial, 1);
    }

    #[test]
    fn test_zone_add_record() {
        let mut zone = DNSZone::new("example.com", "ns1.example.com", "admin@example.com");

        let initial_serial = zone.serial;
        zone.add_record("record-1");

        assert_eq!(zone.record_count(), 1);
        assert_eq!(zone.serial, initial_serial + 1);
    }

    #[test]
    fn test_zone_increment_serial() {
        let mut zone = DNSZone::new("example.com", "ns1.example.com", "admin@example.com");

        assert_eq!(zone.serial, 1);

        zone.increment_serial();
        assert_eq!(zone.serial, 2);
    }

    #[test]
    fn test_dns_manager() {
        let mut manager = DNSManager::new();

        let zone = DNSZone::new("example.com", "ns1.example.com", "admin@example.com");
        manager.add_zone(zone);

        assert_eq!(manager.zone_count(), 1);
    }

    #[test]
    fn test_manager_add_record() {
        let mut manager = DNSManager::new();

        let record = DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 300).unwrap();
        manager.add_record(record);

        assert_eq!(manager.record_count(), 1);
    }

    #[test]
    fn test_manager_records_by_type() {
        let mut manager = DNSManager::new();

        manager.add_record(
            DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 300).unwrap(),
        );
        manager.add_record(
            DNSRecord::new("mail.example.com", RecordType::MX, "mail.example.com", 300).unwrap(),
        );
        manager.add_record(
            DNSRecord::new("www.example.com", RecordType::A, "192.168.1.2", 300).unwrap(),
        );

        let a_records = manager.records_by_type(&RecordType::A);
        assert_eq!(a_records.len(), 2);
    }

    #[test]
    fn test_manager_find_record_by_name() {
        let mut manager = DNSManager::new();

        manager.add_record(
            DNSRecord::new("example.com", RecordType::A, "192.168.1.1", 300).unwrap(),
        );
        manager.add_record(
            DNSRecord::new("example.com", RecordType::AAAA, "2001:db8::1", 300).unwrap(),
        );
        manager.add_record(
            DNSRecord::new("www.example.com", RecordType::A, "192.168.1.2", 300).unwrap(),
        );

        let records = manager.find_record_by_name("example.com");
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_record_type_equality() {
        assert_eq!(RecordType::A, RecordType::A);
        assert_ne!(RecordType::A, RecordType::AAAA);
    }
}
