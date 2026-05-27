// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Command handlers - extracted from lib.rs for maintainability
// Each submodule handles a group of related CLI commands.

pub mod api;
pub mod automation;
pub mod backup;
pub mod catalog;
pub mod cost;
pub mod crds;
pub mod devexp;
pub mod gitops;
pub mod infra;
pub mod multitenancy;
pub mod observability;
pub mod profiles;
pub mod security;
pub mod vm;
