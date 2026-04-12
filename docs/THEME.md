# VMRogue Theme Implementation Guide

This document provides implementation details for the VMRogue TUI and CLI theme system, inspired by GuestKit's design patterns.

## File Structure

```
vmrogue/
├── src/
│   └── tui/
│       ├── mod.rs           # Module exports
│       ├── theme.rs         # Theme colors and styles (400+ lines)
│       ├── config.rs        # Configuration system (250+ lines)
│       └── colors.rs        # CLI color helpers (150+ lines)
├── examples/
│   └── tui.toml            # Sample configuration file
├── scripts/
│   └── test_theme.sh       # Terminal color test script
└── THEME_DESIGN.md         # Complete design specification

Total: ~800 lines of theme infrastructure code
```

## Quick Start

### 1. Test Theme Colors

Run the color test script to see all theme colors in your terminal:

```bash
./scripts/test_theme.sh
```

This will display:
- All color swatches with hex codes
- VM status symbols with colors
- Sample VM list with themed output
- Resource usage bars
- Border examples

### 2. Using the Theme in TUI Code

```rust
use vmrogue::tui::Theme;

let theme = Theme::default();

// Get VM status styling
let style = theme.vm_status_style("running");

// Get VM status symbol with color
let (symbol, style) = theme.vm_status_symbol("running");
// symbol = "●", style = green color

// Get common styles
let title_style = theme.title_style();      // Purple + Bold
let label_style = theme.label_style();      // Light purple
let value_style = theme.value_style();      // White + Bold
let border_style = theme.border_style();    // Dark purple

// Resource-specific styles
let cpu_style = theme.cpu_style();          // Green
let memory_style = theme.memory_style();    // Blue
let disk_style = theme.disk_style();        // Magenta
let network_style = theme.network_style();  // Cyan

// Namespace coloring
let ns_style = theme.namespace_style("default");      // Blue
let ns_style = theme.namespace_style("kube-system");  // Red
let ns_style = theme.namespace_style("my-app");       // Green

// Resource usage with dynamic coloring
let usage_style = theme.resource_usage_style(50.0);  // Green (< 70%)
let usage_style = theme.resource_usage_style(85.0);  // Orange (70-90%)
let usage_style = theme.resource_usage_style(95.0);  // Red (> 90%)
```

### 3. Using CLI Colors

For standard CLI output (not TUI), use the color helpers:

```rust
use vmrogue::tui::colors::cli;
use vmrogue::tui::colors::{vm_status_symbol, resource_bar};

// Format text with colors
println!("{}", cli::header("VMRogue Dashboard"));
println!("VM: {}", cli::vm_name("web-server"));
println!("Namespace: {}", cli::namespace("default"));
println!("Status: {}", cli::vm_status("running"));

// Success/Error/Warning messages
println!("{}", cli::success("VM created successfully"));
println!("{}", cli::error("Failed to delete VM"));
println!("{}", cli::warning("VM is using high memory"));
println!("{}", cli::info("Connecting to cluster..."));

// Resources
println!("CPU: {}", cli::resource("4", "cpu"));
println!("Memory: {}", cli::resource("8Gi", "memory"));

// Status symbol
println!("{} Running", vm_status_symbol("running"));  // ● Running (green)

// Resource bar
let bar = resource_bar(75.0, 20);
println!("CPU: [{}] 75%", bar);
```

### 4. Loading Configuration

```rust
use vmrogue::tui::TuiConfig;

// Load from default location (~/.config/vmrogue/tui.toml)
let config = TuiConfig::load().unwrap_or_default();

// Access settings
if config.ui.show_splash {
    // Show splash screen for splash_duration_ms
}

if config.behavior.confirm_delete {
    // Show confirmation prompt before deleting
}

if config.keybindings.vim_mode {
    // Enable hjkl navigation
}

// Save configuration
config.save()?;
```

## Configuration File

Users can customize the TUI by creating `~/.config/vmrogue/tui.toml`:

```toml
[theme]
name = "default"

[ui]
show_splash = true
splash_duration_ms = 800
show_stats_bar = true
default_view = "dashboard"
auto_refresh_interval = 5
table_style = "rounded"

[behavior]
confirm_delete = true
confirm_stop = false
confirm_restart = false
search_case_sensitive = false
search_regex = false
scroll_amount = 10
max_list_items = 1000

[keybindings]
vim_mode = true
quick_jump = true
```

See `examples/tui.toml` for a complete example with comments.

## Color Palette Reference

### Primary Colors (Purple Theme)
- **PRIMARY**: `#8856DE` (136, 86, 222) - Main brand color
- **DARK_PRIMARY**: `#6A3CB4` (106, 60, 180) - Borders, secondary
- **LIGHT_PRIMARY**: `#A573FF` (165, 115, 255) - Labels, highlights

### Status Colors
- **SUCCESS**: `#32CD32` (50, 205, 50) - LimeGreen
- **WARNING**: `#FFA500` (255, 165, 0) - Orange
- **ERROR**: `#DC322F` (220, 50, 47) - Red
- **INFO**: `#6496FF` (100, 150, 255) - Soft Blue

### VM State Colors
- **VM_RUNNING**: `#32CD32` (green) with symbol `●`
- **VM_PENDING**: `#FFC864` (yellow) with symbol `◐`
- **VM_STOPPED**: `#969696` (gray) with symbol `○`
- **VM_FAILED**: `#DC322F` (red) with symbol `✗`
- **VM_MIGRATING**: `#64C8FF` (cyan) with symbol `⟳`
- **VM_PAUSED**: `#FFA500` (orange) with symbol `⏸`

### Resource Colors
- **CPU**: `#64C864` (100, 200, 100) - Green
- **MEMORY**: `#6496FF` (100, 150, 255) - Blue
- **DISK**: `#DC78FF` (220, 120, 255) - Magenta
- **NETWORK**: `#64DCDC` (100, 220, 220) - Cyan

### Text Colors
- **TEXT_PRIMARY**: `#DCDCDC` (220, 220, 220) - Soft white
- **TEXT_SECONDARY**: `#B4B4B4` (180, 180, 180) - Dimmed
- **TEXT_MUTED**: `#787878` (120, 120, 120) - Muted

### Namespace Colors
- **Default namespace**: `#6496FF` (blue)
- **System namespaces** (kube-*): `#FF6464` (red)
- **User namespaces**: `#96FF96` (green)

### OS Colors (for Templates)
- **Ubuntu**: `#E95420` (233, 84, 32)
- **Fedora**: `#3369AD` (51, 105, 173)
- **CentOS**: `#9C429D` (156, 66, 157)
- **Debian**: `#D70A53` (215, 10, 83)
- **Windows**: `#0078D4` (0, 120, 212)
- **RHEL**: `#EE0000` (238, 0, 0)

## Design Patterns from GuestKit

### 1. Centralized Color Constants
All colors are defined in one place (`theme.rs`) and reused throughout the application.

### 2. Style Helper Methods
Common style combinations (title, label, value) are pre-defined as helper methods.

### 3. Dynamic Coloring
Colors change based on state (VM status, resource usage percentage, namespace type).

### 4. Consistent Typography
Typography modifiers (BOLD, UNDERLINED, ITALIC) are used consistently:
- **BOLD**: Titles, important values, active states
- **UNDERLINED**: Active tabs, highlighted selections
- **ITALIC**: Help text, disclaimers

### 5. Symbol + Color + Text
Status indicators use three signals for accessibility:
- Symbol: Visual shape (●, ○, ✗)
- Color: Semantic meaning (green=good, red=bad)
- Text: Explicit state name ("Running", "Failed")

### 6. Configuration System
Theme is configurable via TOML file at `~/.config/vmrogue/tui.toml`.

### 7. OS-Specific Branding
Template views use OS-specific brand colors (Ubuntu orange, Fedora blue, etc.).

## Testing

### Unit Tests

The theme module includes unit tests:

```bash
cargo test --lib tui::theme
cargo test --lib tui::config
cargo test --lib tui::colors
```

### Visual Testing

Run the color test script:

```bash
./scripts/test_theme.sh
```

Verify that:
- All colors render correctly in your terminal
- RGB values match the design specification
- Status symbols display properly
- Borders and separators are visible

### Integration Testing

When building TUI views:
1. Use the test script to verify colors
2. Test in different terminal emulators
3. Test with different terminal color schemes (light/dark)
4. Verify accessibility (contrast ratios)

## Future Enhancements

### Phase 1 (Current)
- ✅ Core theme system with color constants
- ✅ Configuration file support
- ✅ CLI color helpers
- ✅ VM status styling
- ✅ Resource coloring
- ✅ Namespace coloring

### Phase 2 (Next)
- [ ] Actual TUI views implementation
- [ ] Layout system with ratatui
- [ ] Keyboard navigation
- [ ] Modal dialogs
- [ ] Help overlay

### Phase 3 (Future)
- [ ] Multiple theme support (dark, light, kubernetes)
- [ ] Custom theme creation
- [ ] Theme marketplace
- [ ] Colorblind modes
- [ ] ASCII-only mode

## Dependencies

All theme dependencies are already added to `Cargo.toml`:

```toml
# TUI framework
ratatui = "0.28"
crossterm = "0.28"

# CLI colors
colored = "3.1"
owo-colors = "4.0"

# Configuration
toml = "0.8"
dirs = "5.0"

# Enhanced errors
miette = { version = "7.0", features = ["fancy"] }

# Interactive input
rustyline = "17.0"
```

## Examples

### Example: VM List with Theme

```rust
use ratatui::widgets::{Block, Borders, List, ListItem};
use vmrogue::tui::Theme;

let theme = Theme::default();

// Create list items with themed colors
let items: Vec<ListItem> = vms
    .iter()
    .map(|vm| {
        let (symbol, symbol_style) = theme.vm_status_symbol(&vm.status);
        let name_style = theme.value_style();
        let ns_style = theme.namespace_style(&vm.namespace);

        let content = format!(
            "{} {} ({})",
            symbol,
            vm.name,
            vm.namespace
        );

        ListItem::new(content).style(name_style)
    })
    .collect();

// Create themed block
let block = Block::default()
    .title("Virtual Machines")
    .title_style(theme.title_style())
    .borders(Borders::ALL)
    .border_style(theme.border_style());

let list = List::new(items).block(block);
```

### Example: Resource Gauge

```rust
use ratatui::widgets::Gauge;
use vmrogue::tui::Theme;

let theme = Theme::default();

let cpu_usage = 75.0;
let gauge_style = theme.resource_usage_style(cpu_usage);

let gauge = Gauge::default()
    .block(
        Block::default()
            .title("CPU Usage")
            .title_style(theme.title_style())
            .borders(Borders::ALL)
            .border_style(theme.border_style())
    )
    .gauge_style(gauge_style)
    .percent(cpu_usage as u16)
    .label(format!("{}%", cpu_usage));
```

## Best Practices

1. **Always use theme constants** - Never hardcode RGB values
2. **Use style helpers** - Prefer `theme.title_style()` over manual Style creation
3. **Test colors** - Run `test_theme.sh` after theme changes
4. **Consider accessibility** - Use symbol + color + text for status
5. **Respect user config** - Load and honor TuiConfig settings
6. **Consistent modifiers** - Follow the typography guidelines
7. **Resource-specific colors** - Use dedicated colors for CPU, memory, disk, network

## Documentation

- **THEME_DESIGN.md**: Complete design specification (600+ lines)
- **THEME_IMPLEMENTATION.md**: This file - implementation guide
- **examples/tui.toml**: Sample configuration with comments
- **src/tui/theme.rs**: API documentation in code comments
- **src/tui/config.rs**: Configuration structure documentation

## Inspiration Sources

- **GuestKit**: Color palette system, configuration architecture, view patterns
- **Kubernetes**: Purple brand color inspiration
- **k9s**: Keyboard navigation, real-time updates
- **lazydocker**: Status indicators, interactive patterns
- **Ratatui examples**: Component implementations

## Support

For questions or issues:
1. Check THEME_DESIGN.md for design decisions
2. Run `test_theme.sh` to verify color rendering
3. Review examples in this guide
4. Check unit tests in theme.rs, config.rs, colors.rs

## Changelog

### v0.1.0 (Initial Implementation)
- Core theme system with purple color palette
- Configuration system with TOML support
- CLI color helpers
- VM status styling with symbols
- Resource usage coloring
- Namespace-based coloring
- OS-specific template colors
- Test script for color verification
- Comprehensive documentation
