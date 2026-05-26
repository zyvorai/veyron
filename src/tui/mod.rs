// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// TUI module for interactive terminal interface

pub mod app;
pub mod colors;
pub mod config;
pub mod interactive_app;
pub mod splash;
pub mod state;
pub mod theme;
pub mod ui;
pub mod widgets;

pub use app::App;
pub use config::{BehaviorConfig, KeybindingsConfig, TuiConfig, UiConfig};
pub use interactive_app::InteractiveApp;
pub use splash::SplashScreen;
pub use state::AppState;
pub use theme::Theme;
