// VM Health Check System - Automated diagnostics and recommendations
// This is an innovative feature for proactive VM management

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMHealthReport {
    pub vm_name: String,
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub score: u8, // 0-100
    pub recommendations: Vec<String>,
}

impl VMHealthReport {
    pub fn new(vm_name: String) -> Self {
        Self {
            vm_name,
            overall_status: HealthStatus::Unknown,
            checks: Vec::new(),
            score: 0,
            recommendations: Vec::new(),
        }
    }

    /// Add a health check result
    pub fn add_check(&mut self, check: HealthCheck) {
        self.checks.push(check);
        self.calculate_status();
    }

    /// Calculate overall health status and score
    fn calculate_status(&mut self) {
        if self.checks.is_empty() {
            self.overall_status = HealthStatus::Unknown;
            self.score = 0;
            return;
        }

        let mut critical_count = 0;
        let mut warning_count = 0;
        let mut healthy_count = 0;

        for check in &self.checks {
            match check.status {
                HealthStatus::Critical => critical_count += 1,
                HealthStatus::Warning => warning_count += 1,
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Unknown => {}
            }
        }

        // Determine overall status
        if critical_count > 0 {
            self.overall_status = HealthStatus::Critical;
        } else if warning_count > 0 {
            self.overall_status = HealthStatus::Warning;
        } else if healthy_count > 0 {
            self.overall_status = HealthStatus::Healthy;
        } else {
            self.overall_status = HealthStatus::Unknown;
        }

        // Calculate score (0-100)
        let total_checks = self.checks.len() as f64;
        let health_score = (healthy_count as f64 / total_checks) * 100.0;
        let warning_penalty = (warning_count as f64 / total_checks) * 20.0;
        let critical_penalty = (critical_count as f64 / total_checks) * 50.0;

        self.score = (health_score - warning_penalty - critical_penalty)
            .clamp(0.0, 100.0)
            .round() as u8;

        // Collect recommendations
        self.recommendations = self
            .checks
            .iter()
            .filter_map(|c| c.recommendation.clone())
            .collect();
    }

    /// Generate health check for VM resource allocation
    pub fn check_resources(cpu: u32, memory: &str, disk: &str) -> Vec<HealthCheck> {
        let mut checks = Vec::new();

        // CPU check
        if cpu == 1 {
            checks.push(HealthCheck {
                name: "CPU Allocation".to_string(),
                status: HealthStatus::Warning,
                message: "Single CPU core may limit performance".to_string(),
                recommendation: Some(
                    "Consider allocating at least 2 CPU cores for better performance".to_string(),
                ),
            });
        } else if cpu >= 8 {
            checks.push(HealthCheck {
                name: "CPU Allocation".to_string(),
                status: HealthStatus::Healthy,
                message: format!(
                    "{} CPU cores allocated - excellent for demanding workloads",
                    cpu
                ),
                recommendation: None,
            });
        } else {
            checks.push(HealthCheck {
                name: "CPU Allocation".to_string(),
                status: HealthStatus::Healthy,
                message: format!("{} CPU cores allocated - good", cpu),
                recommendation: None,
            });
        }

        // Memory check
        let memory_gb = parse_memory_size(memory);
        if memory_gb < 2.0 {
            checks.push(HealthCheck {
                name: "Memory Allocation".to_string(),
                status: HealthStatus::Warning,
                message: format!("{} memory is low", memory),
                recommendation: Some(
                    "Consider allocating at least 2Gi memory for stability".to_string(),
                ),
            });
        } else if memory_gb >= 16.0 {
            checks.push(HealthCheck {
                name: "Memory Allocation".to_string(),
                status: HealthStatus::Healthy,
                message: format!(
                    "{} memory - excellent for memory-intensive workloads",
                    memory
                ),
                recommendation: None,
            });
        } else {
            checks.push(HealthCheck {
                name: "Memory Allocation".to_string(),
                status: HealthStatus::Healthy,
                message: format!("{} memory allocated - good", memory),
                recommendation: None,
            });
        }

        // Disk check
        let disk_gb = parse_disk_size(disk);
        if disk_gb < 10.0 {
            checks.push(HealthCheck {
                name: "Disk Space".to_string(),
                status: HealthStatus::Warning,
                message: format!("{} disk space is limited", disk),
                recommendation: Some("Consider allocating at least 20Gi disk space".to_string()),
            });
        } else if disk_gb >= 100.0 {
            checks.push(HealthCheck {
                name: "Disk Space".to_string(),
                status: HealthStatus::Healthy,
                message: format!(
                    "{} disk space - excellent for data-intensive applications",
                    disk
                ),
                recommendation: None,
            });
        } else {
            checks.push(HealthCheck {
                name: "Disk Space".to_string(),
                status: HealthStatus::Healthy,
                message: format!("{} disk space allocated - good", disk),
                recommendation: None,
            });
        }

        checks
    }

    /// Check if resource allocation matches workload type
    pub fn check_workload_match(cpu: u32, memory: &str, workload: &str) -> HealthCheck {
        let memory_gb = parse_memory_size(memory);

        match workload.to_lowercase().as_str() {
            "database" => {
                if cpu >= 4 && memory_gb >= 8.0 {
                    HealthCheck {
                        name: "Workload Match".to_string(),
                        status: HealthStatus::Healthy,
                        message: "Resources well-suited for database workload".to_string(),
                        recommendation: None,
                    }
                } else {
                    HealthCheck {
                        name: "Workload Match".to_string(),
                        status: HealthStatus::Warning,
                        message: "Database workloads typically need more resources".to_string(),
                        recommendation: Some(
                            "Recommended: 4+ CPU cores and 8Gi+ memory for databases".to_string(),
                        ),
                    }
                }
            }
            "web" => {
                if cpu >= 2 && memory_gb >= 4.0 {
                    HealthCheck {
                        name: "Workload Match".to_string(),
                        status: HealthStatus::Healthy,
                        message: "Resources appropriate for web server".to_string(),
                        recommendation: None,
                    }
                } else {
                    HealthCheck {
                        name: "Workload Match".to_string(),
                        status: HealthStatus::Warning,
                        message: "Web servers should have adequate resources".to_string(),
                        recommendation: Some(
                            "Recommended: 2+ CPU cores and 4Gi+ memory for web servers".to_string(),
                        ),
                    }
                }
            }
            "development" => HealthCheck {
                name: "Workload Match".to_string(),
                status: HealthStatus::Healthy,
                message: "Resources suitable for development".to_string(),
                recommendation: None,
            },
            _ => HealthCheck {
                name: "Workload Match".to_string(),
                status: HealthStatus::Healthy,
                message: "General purpose resource allocation".to_string(),
                recommendation: None,
            },
        }
    }
}

/// Parse memory size to GB
fn parse_memory_size(memory: &str) -> f64 {
    let memory_upper = memory.to_uppercase();
    if let Some(value) = memory_upper.strip_suffix("GI") {
        value.parse::<f64>().unwrap_or(0.0)
    } else if let Some(value) = memory_upper.strip_suffix("MI") {
        value.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if let Some(value) = memory_upper.strip_suffix("G") {
        value.parse::<f64>().unwrap_or(0.0)
    } else if let Some(value) = memory_upper.strip_suffix("M") {
        value.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else {
        memory.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Parse disk size to GB
fn parse_disk_size(disk: &str) -> f64 {
    let disk_upper = disk.to_uppercase();
    if let Some(value) = disk_upper.strip_suffix("GI") {
        value.parse::<f64>().unwrap_or(0.0)
    } else if let Some(value) = disk_upper.strip_suffix("TI") {
        value.parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if let Some(value) = disk_upper.strip_suffix("G") {
        value.parse::<f64>().unwrap_or(0.0)
    } else if let Some(value) = disk_upper.strip_suffix("T") {
        value.parse::<f64>().unwrap_or(0.0) * 1024.0
    } else {
        disk.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_report() {
        let mut report = VMHealthReport::new("test-vm".to_string());

        report.add_check(HealthCheck {
            name: "Test".to_string(),
            status: HealthStatus::Healthy,
            message: "All good".to_string(),
            recommendation: None,
        });

        assert_eq!(report.overall_status, HealthStatus::Healthy);
        assert!(report.score > 80);
    }

    #[test]
    fn test_resource_checks() {
        let checks = VMHealthReport::check_resources(2, "4Gi", "20Gi");
        assert_eq!(checks.len(), 3);
        assert!(checks
            .iter()
            .all(|c| c.status == HealthStatus::Healthy || c.status == HealthStatus::Warning));
    }

    #[test]
    fn test_parse_memory_size() {
        assert_eq!(parse_memory_size("4Gi"), 4.0);
        assert_eq!(parse_memory_size("2048Mi"), 2.0);
        assert_eq!(parse_memory_size("8G"), 8.0);
    }

    #[test]
    fn test_workload_match() {
        let check = VMHealthReport::check_workload_match(4, "8Gi", "database");
        assert_eq!(check.status, HealthStatus::Healthy);

        let check = VMHealthReport::check_workload_match(1, "1Gi", "database");
        assert_eq!(check.status, HealthStatus::Warning);
    }
}
