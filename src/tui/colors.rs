// Color helpers for CLI and TUI
// CLI colors use colored/owo_colors for terminal output
// TUI colors use ratatui::style::Color for TUI rendering

use colored::Colorize;
use ratatui::style::Color;

/// TUI color palette - Coral-Terracotta Orange theme (Pantone 7416 C)
/// Matches GuestKit's visual identity
pub mod tui {
    use super::Color;

    // Primary brand colors - Coral-Terracotta Orange (Pantone 7416 C)
    pub const ORANGE: Color = Color::Rgb(222, 115, 86); // #DE7356 Primary coral orange
    pub const DARK_ORANGE: Color = Color::Rgb(180, 85, 60); // #B4553C Darker terracotta
    pub const LIGHT_ORANGE: Color = Color::Rgb(255, 145, 115); // #FF9173 Lighter coral

    // Aliases for compatibility
    pub const PRIMARY: Color = ORANGE;
    pub const DARK_PRIMARY: Color = DARK_ORANGE;
    pub const LIGHT_PRIMARY: Color = LIGHT_ORANGE;

    // Status colors
    pub const SUCCESS: Color = Color::Rgb(50, 205, 50); // #32CD32 Brighter green
    pub const WARNING: Color = Color::Rgb(255, 200, 0); // #FFC800 Deeper yellow
    pub const ERROR: Color = Color::Rgb(220, 50, 47); // #DC322F Deep red
    pub const INFO: Color = Color::Rgb(100, 150, 255); // #6496FF Soft blue

    // Text colors
    pub const TEXT: Color = Color::Rgb(220, 220, 220); // #DCDCDC Softer white
    pub const TEXT_MUTED: Color = Color::Rgb(128, 128, 128); // #808080

    // UI element colors
    pub const BORDER: Color = DARK_ORANGE;
    pub const BG: Color = Color::Reset;
}

/// Gradient system for TUI rendering
pub mod gradient {
    use super::Color;
    use ratatui::style::Style;
    use ratatui::text::Span;

    /// A color gradient defined by a series of color stops
    #[derive(Debug, Clone)]
    pub struct Gradient {
        pub stops: Vec<(f64, u8, u8, u8)>, // (position 0.0–1.0, r, g, b)
    }

    impl Gradient {
        pub fn new(stops: Vec<(f64, u8, u8, u8)>) -> Self {
            Self { stops }
        }

        /// Interpolate to get a color at position t (0.0–1.0)
        pub fn at(&self, t: f64) -> Color {
            let t = t.clamp(0.0, 1.0);
            if self.stops.is_empty() {
                return Color::White;
            }
            if self.stops.len() == 1 {
                let (_, r, g, b) = self.stops[0];
                return Color::Rgb(r, g, b);
            }

            // Find the two stops to interpolate between
            let mut lower = &self.stops[0];
            let mut upper = &self.stops[self.stops.len() - 1];

            for i in 0..self.stops.len() - 1 {
                if t >= self.stops[i].0 && t <= self.stops[i + 1].0 {
                    lower = &self.stops[i];
                    upper = &self.stops[i + 1];
                    break;
                }
            }

            let range = upper.0 - lower.0;
            let local_t = if range > 0.0 {
                (t - lower.0) / range
            } else {
                0.0
            };

            let r = lerp_u8(lower.1, upper.1, local_t);
            let g = lerp_u8(lower.2, upper.2, local_t);
            let b = lerp_u8(lower.3, upper.3, local_t);

            Color::Rgb(r, g, b)
        }

        /// Generate N evenly-spaced colors from the gradient
        pub fn colors(&self, n: usize) -> Vec<Color> {
            if n == 0 {
                return Vec::new();
            }
            if n == 1 {
                return vec![self.at(0.5)];
            }
            (0..n).map(|i| self.at(i as f64 / (n - 1) as f64)).collect()
        }

        /// Render a text string with per-character gradient coloring
        pub fn text<'a>(&self, text: &'a str) -> Vec<Span<'a>> {
            if text.is_empty() {
                return Vec::new();
            }
            let chars: Vec<char> = text.chars().collect();
            let n = chars.len();
            chars
                .into_iter()
                .enumerate()
                .map(|(i, ch)| {
                    let t = if n > 1 {
                        i as f64 / (n - 1) as f64
                    } else {
                        0.5
                    };
                    let mut buf = [0u8; 4];
                    let s = ch.encode_utf8(&mut buf);
                    // We need owned strings for Span, so use Span::raw with String
                    Span::styled(s.to_string(), Style::default().fg(self.at(t)))
                })
                .collect()
        }

        /// Render a bar (gauge fill) with gradient colors
        pub fn bar(&self, width: usize, fill_ratio: f64) -> Vec<Span<'static>> {
            let filled = (fill_ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
            let empty = width.saturating_sub(filled);
            let mut spans = Vec::with_capacity(filled + 1);
            for i in 0..filled {
                let t = if filled > 1 {
                    i as f64 / (filled - 1) as f64
                } else {
                    0.5
                };
                spans.push(Span::styled(
                    "█".to_string(),
                    Style::default().fg(self.at(t)),
                ));
            }
            if empty > 0 {
                spans.push(Span::styled(
                    "░".repeat(empty),
                    Style::default().fg(Color::Rgb(60, 60, 60)),
                ));
            }
            spans
        }

        /// Render a horizontal gradient border/line
        pub fn line(&self, width: usize, ch: char) -> Vec<Span<'static>> {
            (0..width)
                .map(|i| {
                    let t = if width > 1 {
                        i as f64 / (width - 1) as f64
                    } else {
                        0.5
                    };
                    let mut buf = [0u8; 4];
                    let s = ch.encode_utf8(&mut buf).to_string();
                    Span::styled(s, Style::default().fg(self.at(t)))
                })
                .collect()
        }
    }

    fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
        let result = a as f64 + (b as f64 - a as f64) * t;
        result.round().clamp(0.0, 255.0) as u8
    }

    // ── Pre-built Gradient Presets ──────────────────────────────────

    /// Coral sunset: dark terracotta → coral → light peach
    pub fn sunset() -> Gradient {
        Gradient::new(vec![
            (0.0, 140, 60, 35),   // Deep terracotta
            (0.3, 222, 115, 86),  // Coral orange (primary)
            (0.6, 255, 145, 115), // Light coral
            (1.0, 255, 200, 170), // Warm peach
        ])
    }

    /// Fire: deep red → orange → golden yellow
    pub fn fire() -> Gradient {
        Gradient::new(vec![
            (0.0, 180, 30, 20),  // Deep ember
            (0.3, 220, 80, 40),  // Burnt orange
            (0.6, 255, 160, 50), // Amber
            (1.0, 255, 220, 80), // Golden
        ])
    }

    /// Ocean: deep blue → teal → cyan
    pub fn ocean() -> Gradient {
        Gradient::new(vec![
            (0.0, 20, 60, 140),   // Deep ocean
            (0.3, 40, 100, 180),  // Blue
            (0.6, 60, 160, 200),  // Teal
            (1.0, 100, 220, 240), // Cyan
        ])
    }

    /// Health: red → orange → yellow → green (for usage gauges)
    pub fn health() -> Gradient {
        Gradient::new(vec![
            (0.0, 50, 205, 50),   // Green (low usage = good)
            (0.5, 255, 200, 0),   // Yellow (medium)
            (0.75, 255, 130, 50), // Orange (high)
            (1.0, 220, 50, 47),   // Red (critical)
        ])
    }

    /// Health inverted: green → yellow → red (for score/rating gauges where high = good)
    pub fn health_inverted() -> Gradient {
        Gradient::new(vec![
            (0.0, 220, 50, 47), // Red (low score = bad)
            (0.5, 255, 200, 0), // Yellow
            (1.0, 50, 205, 50), // Green (high score = good)
        ])
    }

    /// Aurora: purple → magenta → pink → coral
    pub fn aurora() -> Gradient {
        Gradient::new(vec![
            (0.0, 100, 80, 180),  // Deep purple
            (0.3, 160, 80, 180),  // Magenta
            (0.6, 220, 100, 150), // Pink
            (1.0, 255, 145, 115), // Coral
        ])
    }

    /// Neon: cyan → blue → purple → pink
    pub fn neon() -> Gradient {
        Gradient::new(vec![
            (0.0, 0, 255, 200),   // Neon cyan
            (0.33, 80, 120, 255), // Electric blue
            (0.66, 160, 80, 255), // Purple
            (1.0, 255, 80, 180),  // Hot pink
        ])
    }

    /// Frost: white → light blue → ice blue
    pub fn frost() -> Gradient {
        Gradient::new(vec![
            (0.0, 200, 220, 240), // Pale ice
            (0.5, 140, 180, 220), // Light blue
            (1.0, 80, 140, 200),  // Ice blue
        ])
    }

    /// Terminal green: dark green → bright green (Matrix style)
    pub fn matrix() -> Gradient {
        Gradient::new(vec![
            (0.0, 0, 80, 0),    // Dark green
            (0.5, 0, 180, 0),   // Green
            (1.0, 50, 255, 50), // Bright green
        ])
    }

    /// Brand gradient: the VMRogue coral gradient for headers and titles
    pub fn brand() -> Gradient {
        Gradient::new(vec![
            (0.0, 180, 85, 60),   // Dark terracotta
            (0.4, 222, 115, 86),  // Coral orange
            (0.7, 255, 145, 115), // Light coral
            (1.0, 255, 180, 140), // Peach highlight
        ])
    }

    /// Security gradient: green → yellow → red for severity
    pub fn severity() -> Gradient {
        Gradient::new(vec![
            (0.0, 50, 205, 50), // Info (green)
            (0.25, 100, 200, 80),
            (0.5, 255, 200, 0), // Warning (yellow)
            (0.75, 255, 130, 50),
            (1.0, 220, 50, 47), // Critical (red)
        ])
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_gradient_at() {
            let g = sunset();
            match g.at(0.0) {
                Color::Rgb(r, _, _) => assert_eq!(r, 140),
                _ => panic!("Expected Rgb"),
            }
        }

        #[test]
        fn test_gradient_colors() {
            let g = fire();
            let colors = g.colors(5);
            assert_eq!(colors.len(), 5);
        }

        #[test]
        fn test_gradient_text() {
            let g = brand();
            let spans = g.text("VMRogue");
            assert_eq!(spans.len(), 7);
        }

        #[test]
        fn test_gradient_bar() {
            let g = health();
            let spans = g.bar(20, 0.75);
            assert!(!spans.is_empty());
        }

        #[test]
        fn test_lerp() {
            assert_eq!(lerp_u8(0, 255, 0.5), 128);
            assert_eq!(lerp_u8(100, 200, 0.0), 100);
            assert_eq!(lerp_u8(100, 200, 1.0), 200);
        }
    }
}

/// Color scheme for CLI output
pub mod cli {
    use super::*;

    /// Format VM name
    pub fn vm_name(name: &str) -> String {
        name.bright_white().bold().to_string()
    }

    /// Format namespace
    pub fn namespace(ns: &str) -> String {
        if ns == "default" {
            ns.bright_blue().to_string()
        } else if ns.starts_with("kube-") {
            ns.bright_red().to_string()
        } else {
            ns.bright_green().to_string()
        }
    }

    /// Format VM status
    pub fn vm_status(status: &str) -> String {
        match status.to_lowercase().as_str() {
            "running" => status.bright_green().bold().to_string(),
            "pending" | "scheduling" => status.bright_yellow().to_string(),
            "stopped" | "terminated" => status.bright_black().to_string(),
            "failed" | "error" => status.bright_red().bold().to_string(),
            "migrating" => status.bright_cyan().to_string(),
            "paused" => status.bright_magenta().to_string(),
            _ => status.dimmed().to_string(),
        }
    }

    /// Format success message
    pub fn success(msg: &str) -> String {
        format!("{} {}", "✓".bright_green().bold(), msg.bright_green())
    }

    /// Format error message
    pub fn error(msg: &str) -> String {
        format!("{} {}", "✗".bright_red().bold(), msg.bright_red())
    }

    /// Format warning message
    pub fn warning(msg: &str) -> String {
        format!("{} {}", "⚠".bright_yellow().bold(), msg.bright_yellow())
    }

    /// Format info message
    pub fn info(msg: &str) -> String {
        format!("{} {}", "ℹ".bright_blue().bold(), msg.bright_blue())
    }

    /// Format resource value (CPU, Memory, etc.)
    pub fn resource(value: &str, resource_type: &str) -> String {
        match resource_type {
            "cpu" => value.bright_green().to_string(),
            "memory" => value.bright_blue().to_string(),
            "disk" => value.bright_magenta().to_string(),
            "network" => value.bright_cyan().to_string(),
            _ => value.white().to_string(),
        }
    }

    /// Format header text
    pub fn header(text: &str) -> String {
        text.bright_purple().bold().to_string()
    }

    /// Format label
    pub fn label(text: &str) -> String {
        text.bright_white().to_string()
    }

    /// Format value
    pub fn value(text: &str) -> String {
        text.white().bold().to_string()
    }

    /// Format muted/dimmed text
    pub fn muted(text: &str) -> String {
        text.dimmed().to_string()
    }

    /// Format command/code
    pub fn command(text: &str) -> String {
        text.bright_cyan().to_string()
    }

    /// Format file path
    pub fn path(text: &str) -> String {
        text.bright_yellow().to_string()
    }
}

/// VM status symbols with colors
pub fn vm_status_symbol(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "running" => "●".bright_green().to_string(),
        "pending" | "scheduling" => "◐".bright_yellow().to_string(),
        "stopped" | "terminated" => "○".bright_black().to_string(),
        "failed" | "error" => "✗".bright_red().to_string(),
        "migrating" => "⟳".bright_cyan().to_string(),
        "paused" => "⏸".bright_magenta().to_string(),
        _ => "?".dimmed().to_string(),
    }
}

/// Create a colored bar for resource usage
pub fn resource_bar(percentage: f64, width: usize) -> String {
    let percentage = percentage.clamp(0.0, 100.0);
    let filled = ((percentage / 100.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    let color = if percentage < 70.0 {
        "green"
    } else if percentage < 90.0 {
        "yellow"
    } else {
        "red"
    };

    let filled_str = "█".repeat(filled);
    let empty_str = "░".repeat(empty);

    match color {
        "green" => format!("{}{}", filled_str.bright_green(), empty_str.bright_black()),
        "yellow" => format!("{}{}", filled_str.bright_yellow(), empty_str.bright_black()),
        "red" => format!("{}{}", filled_str.bright_red(), empty_str.bright_black()),
        _ => format!("{}{}", filled_str, empty_str),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_status_symbol() {
        let symbol = vm_status_symbol("running");
        assert!(symbol.contains("●"));

        let symbol = vm_status_symbol("failed");
        assert!(symbol.contains("✗"));
    }

    #[test]
    fn test_resource_bar() {
        let bar = resource_bar(50.0, 10);
        // Should have some filled and some empty chars
        assert!(!bar.is_empty());

        let bar = resource_bar(100.0, 10);
        assert!(!bar.is_empty());
    }
}
