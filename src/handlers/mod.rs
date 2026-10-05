// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Command handlers - extracted from lib.rs for maintainability
// Each submodule handles a group of related CLI commands.

pub mod agent;
pub mod api;
pub mod automation;
pub mod backup;
pub mod catalog;
pub mod copilot;
pub mod cost;
pub mod crds;
pub mod devexp;
pub mod gitops;
pub mod infra;
pub mod multitenancy;
pub mod observability;
pub mod profiles;
pub mod security;
pub mod users;
pub mod vm;
