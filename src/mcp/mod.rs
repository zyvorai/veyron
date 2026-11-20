// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Model Context Protocol: Veyron as an MCP server (`server`), a stdio bridge for
//! desktop clients (`bridge`), and Veyron consuming other MCP servers (`client`).

pub mod bridge;
pub mod client;
pub mod server;
