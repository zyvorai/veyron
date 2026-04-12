// TUI Widgets - Reusable interactive components

pub mod bar_chart;
pub mod dialog;
pub mod footer;
pub mod input;
pub mod menu;
pub mod notification;
pub mod progress;
pub mod quick_jump;
pub mod resource_gauge;
pub mod search_bar;
pub mod sparkline_chart;
pub mod stats_bar;

pub use bar_chart::BarChart;
pub use dialog::{Dialog, DialogType};
pub use footer::{Footer, ViewContext};
pub use input::{InputDialog, InputField};
pub use menu::{Menu, MenuItem};
pub use notification::{Notification, NotificationManager};
pub use progress::ProgressBar;
pub use quick_jump::QuickJumpMenu;
pub use resource_gauge::{MultiGaugePanel, ResourceGauge};
pub use search_bar::{SearchBar, SearchMode};
pub use sparkline_chart::SparklineChart;
pub use stats_bar::StatsBar;
