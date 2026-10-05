// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

pub mod app_config;
pub mod builder;
pub mod types;
pub mod validator;

pub use app_config::AppConfig;
pub use builder::VMConfigBuilder;
pub use types::{
    BootloaderType, CPUConfig, ClockConfig, CloudInitConfig, CloudInitDelivery, DiskConfig,
    DiskDeviceType, DiskSource, FeaturesConfig, FirmwareConfig, HyperVConfig, InterfaceConfig,
    MemoryConfig, NetworkType, SysprepConfig, TimersConfig, VMConfig, VmExposeConfig, VmExposePort,
    VmGpuDevice, VmHostDevice, VmMatcherRef, VmScheduling, VmVgpuOptions, VmVirtioFs,
    VmWatchdogConfig,
};
pub use validator::validate_vm_config;
