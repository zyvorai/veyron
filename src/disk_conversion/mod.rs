// Disk Conversion - Disk format conversion utilities

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConverter {
    pub supported_formats: Vec<DiskFormat>,
    pub conversions: Vec<ConversionJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiskFormat {
    Raw,
    Qcow2,
    Vmdk,
    Vdi,
    Vhd,
    Vhdx,
    Iso,
}

impl DiskFormat {
    pub fn extension(&self) -> &str {
        match self {
            Self::Raw => "img",
            Self::Qcow2 => "qcow2",
            Self::Vmdk => "vmdk",
            Self::Vdi => "vdi",
            Self::Vhd => "vhd",
            Self::Vhdx => "vhdx",
            Self::Iso => "iso",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "img" | "raw" => Some(Self::Raw),
            "qcow2" => Some(Self::Qcow2),
            "vmdk" => Some(Self::Vmdk),
            "vdi" => Some(Self::Vdi),
            "vhd" => Some(Self::Vhd),
            "vhdx" => Some(Self::Vhdx),
            "iso" => Some(Self::Iso),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionJob {
    pub id: String,
    pub source_path: String,
    pub target_path: String,
    pub source_format: DiskFormat,
    pub target_format: DiskFormat,
    pub status: ConversionStatus,
    pub progress_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConversionStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

impl DiskConverter {
    pub fn new() -> Self {
        Self {
            supported_formats: vec![
                DiskFormat::Raw,
                DiskFormat::Qcow2,
                DiskFormat::Vmdk,
                DiskFormat::Vdi,
                DiskFormat::Vhd,
            ],
            conversions: Vec::new(),
        }
    }

    pub fn can_convert(&self, from: &DiskFormat, to: &DiskFormat) -> bool {
        self.supported_formats.contains(from) && self.supported_formats.contains(to) && from != to
    }

    pub fn start_conversion(
        &mut self,
        source: &str,
        target: &str,
        from: DiskFormat,
        to: DiskFormat,
    ) -> Option<String> {
        if !self.can_convert(&from, &to) {
            return None;
        }
        let id = format!("conv-{}", chrono::Utc::now().timestamp_micros());
        self.conversions.push(ConversionJob {
            id: id.clone(),
            source_path: source.to_string(),
            target_path: target.to_string(),
            source_format: from,
            target_format: to,
            status: ConversionStatus::Pending,
            progress_percent: 0.0,
        });
        Some(id)
    }

    pub fn get_job(&self, id: &str) -> Option<&ConversionJob> {
        self.conversions.iter().find(|j| j.id == id)
    }
    pub fn active_jobs(&self) -> Vec<&ConversionJob> {
        self.conversions
            .iter()
            .filter(|j| {
                matches!(
                    j.status,
                    ConversionStatus::InProgress | ConversionStatus::Pending
                )
            })
            .collect()
    }
}

impl Default for DiskConverter {
    fn default() -> Self {
        Self::new()
    }
}
