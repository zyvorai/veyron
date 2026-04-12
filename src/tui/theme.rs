// VMRogue TUI Theme System
// Coral-Terracotta Orange color scheme (Pantone 7416 C inspired)
// Matches GuestKit's visual identity

use ratatui::style::{Color, Modifier, Style};

/// Main theme structure containing all color definitions
#[derive(Debug, Clone)]
pub struct Theme {
    // Primary brand colors (Coral-Terracotta Orange - Pantone 7416 C inspired)
    pub primary: Color,
    pub dark_primary: Color,
    pub light_primary: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // VM state colors
    pub vm_running: Color,
    pub vm_pending: Color,
    pub vm_stopped: Color,
    pub vm_failed: Color,
    pub vm_migrating: Color,
    pub vm_paused: Color,

    // Resource colors
    pub cpu_color: Color,
    pub memory_color: Color,
    pub disk_color: Color,
    pub network_color: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub bg_color: Color,
    pub border_color: Color,

    // Namespace colors
    pub namespace_default: Color,
    pub namespace_system: Color,
    pub namespace_user: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Primary colors - Coral-Terracotta Orange theme (Pantone 7416 C)
            primary: Color::Rgb(222, 115, 86), // #DE7356 Primary coral orange
            dark_primary: Color::Rgb(180, 85, 60), // #B4553C Darker terracotta
            light_primary: Color::Rgb(255, 145, 115), // #FF9173 Lighter coral

            // Status colors
            success: Color::Rgb(50, 205, 50), // #32CD32 Brighter green
            warning: Color::Rgb(255, 200, 0), // #FFC800 Deeper yellow
            error: Color::Rgb(220, 50, 47),   // #DC322F Deep red
            info: Color::Rgb(100, 150, 255),  // #6496FF Soft blue

            // VM state colors
            vm_running: Color::Rgb(50, 205, 50), // SUCCESS - green
            vm_pending: Color::Rgb(255, 200, 0), // WARNING - yellow
            vm_stopped: Color::Rgb(150, 150, 150), // #969696 gray
            vm_failed: Color::Rgb(220, 50, 47),  // ERROR - red
            vm_migrating: Color::Rgb(100, 150, 255), // INFO - blue
            vm_paused: Color::Rgb(255, 200, 0),  // WARNING - yellow

            // Resource colors
            cpu_color: Color::Rgb(50, 205, 50),       // Green
            memory_color: Color::Rgb(100, 150, 255),  // Blue
            disk_color: Color::Rgb(222, 115, 86),     // Orange (primary)
            network_color: Color::Rgb(100, 220, 220), // Cyan

            // Text and background
            text_primary: Color::Rgb(220, 220, 220), // #DCDCDC soft white
            text_secondary: Color::Rgb(180, 180, 180), // #B4B4B4 dimmed
            text_muted: Color::Rgb(128, 128, 128),   // #808080 muted
            bg_color: Color::Reset,                  // Transparent
            border_color: Color::Rgb(180, 85, 60),   // DARK_PRIMARY

            // Namespace colors
            namespace_default: Color::Rgb(100, 150, 255), // Blue
            namespace_system: Color::Rgb(220, 50, 47),    // Red
            namespace_user: Color::Rgb(50, 205, 50),      // Green
        }
    }
}

impl Theme {
    /// Create a new theme with default colors
    pub fn new() -> Self {
        Self::default()
    }

    /// Get style for VM status
    pub fn vm_status_style(&self, status: &str) -> Style {
        let color = match status.to_lowercase().as_str() {
            "running" => self.vm_running,
            "pending" | "scheduling" => self.vm_pending,
            "stopped" | "terminated" => self.vm_stopped,
            "failed" | "error" => self.vm_failed,
            "migrating" => self.vm_migrating,
            "paused" => self.vm_paused,
            _ => self.text_muted,
        };
        Style::default().fg(color)
    }

    /// Get VM status symbol with color
    pub fn vm_status_symbol(&self, status: &str) -> (&str, Style) {
        match status.to_lowercase().as_str() {
            "running" => ("●", Style::default().fg(self.vm_running)),
            "pending" | "scheduling" => ("◐", Style::default().fg(self.vm_pending)),
            "stopped" | "terminated" => ("○", Style::default().fg(self.vm_stopped)),
            "failed" | "error" => ("✗", Style::default().fg(self.vm_failed)),
            "migrating" => ("⟳", Style::default().fg(self.vm_migrating)),
            "paused" => ("⏸", Style::default().fg(self.vm_paused)),
            _ => ("?", Style::default().fg(self.text_muted)),
        }
    }

    /// Get style for namespace based on type
    pub fn namespace_style(&self, namespace: &str) -> Style {
        let color = if namespace == "default" {
            self.namespace_default
        } else if namespace.starts_with("kube-") {
            self.namespace_system
        } else {
            self.namespace_user
        };
        Style::default().fg(color)
    }

    /// Get style for resource usage percentage
    pub fn resource_usage_style(&self, percentage: f64) -> Style {
        let color = if percentage < 70.0 {
            self.success
        } else if percentage < 90.0 {
            self.warning
        } else {
            self.error
        };
        Style::default().fg(color)
    }

    // Common style helpers

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn label_style(&self) -> Style {
        Style::default().fg(self.light_primary)
    }

    pub fn value_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn help_style(&self) -> Style {
        Style::default()
            .fg(self.text_muted)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn active_style(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border_color)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn info_style(&self) -> Style {
        Style::default().fg(self.info)
    }

    pub fn text_style(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    pub fn secondary_style(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Get CPU resource color
    pub fn cpu_style(&self) -> Style {
        Style::default().fg(self.cpu_color)
    }

    /// Get Memory resource color
    pub fn memory_style(&self) -> Style {
        Style::default().fg(self.memory_color)
    }

    /// Get Disk resource color
    pub fn disk_style(&self) -> Style {
        Style::default().fg(self.disk_color)
    }

    /// Get Network resource color
    pub fn network_style(&self) -> Style {
        Style::default().fg(self.network_color)
    }
}

// OS-specific colors for template display
pub mod os_colors {
    use ratatui::style::Color;

    pub const UBUNTU: Color = Color::Rgb(233, 84, 32); // #E95420 Orange
    pub const FEDORA: Color = Color::Rgb(51, 105, 173); // #3369AD Blue
    pub const CENTOS: Color = Color::Rgb(156, 66, 157); // #9C429D Purple
    pub const DEBIAN: Color = Color::Rgb(215, 10, 83); // #D70A53 Red
    pub const WINDOWS: Color = Color::Rgb(0, 120, 212); // #0078D4 Blue
    pub const RHEL: Color = Color::Rgb(238, 0, 0); // #EE0000 Red

    pub fn get_os_color(os_name: &str) -> Color {
        match os_name.to_lowercase().as_str() {
            "ubuntu" => UBUNTU,
            "fedora" => FEDORA,
            "centos" => CENTOS,
            "debian" => DEBIAN,
            "windows" => WINDOWS,
            "rhel" | "redhat" => RHEL,
            _ => Color::Rgb(150, 150, 150), // Default gray
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        // Test Coral-Terracotta Orange theme colors
        assert_eq!(theme.primary, Color::Rgb(222, 115, 86));
        assert_eq!(theme.dark_primary, Color::Rgb(180, 85, 60));
        assert_eq!(theme.light_primary, Color::Rgb(255, 145, 115));
        assert_eq!(theme.success, Color::Rgb(50, 205, 50));
    }

    #[test]
    fn test_vm_status_styles() {
        let theme = Theme::new();
        let (symbol, _) = theme.vm_status_symbol("running");
        assert_eq!(symbol, "●");

        let (symbol, _) = theme.vm_status_symbol("failed");
        assert_eq!(symbol, "✗");
    }

    #[test]
    fn test_namespace_styles() {
        let theme = Theme::new();
        // Just ensure it doesn't panic
        let _ = theme.namespace_style("default");
        let _ = theme.namespace_style("kube-system");
        let _ = theme.namespace_style("my-namespace");
    }

    #[test]
    fn test_resource_usage_styles() {
        let theme = Theme::new();
        // Low usage - should be success (green)
        let style = theme.resource_usage_style(50.0);
        assert_eq!(style.fg, Some(theme.success));

        // High usage - should be warning (orange)
        let style = theme.resource_usage_style(85.0);
        assert_eq!(style.fg, Some(theme.warning));

        // Critical usage - should be error (red)
        let style = theme.resource_usage_style(95.0);
        assert_eq!(style.fg, Some(theme.error));
    }
}
