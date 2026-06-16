# Veyron Interactive TUI Guide

**See also:** [Documentation index](README.md) for related guides (theme, quick reference, deployment).

## Overview

Veyron provides a fully interactive Terminal User Interface (TUI) with dialogs, forms, context menus, and real-time notifications for managing KubeVirt VMs.

## Launching the TUI

### Basic Mode
```bash
# Launch basic TUI
vmrogue tui

# With specific namespace
vmrogue tui --namespace production
```

### Enhanced Interactive Mode
```bash
# Launch with full interactive features
vmrogue tui --interactive

# With custom theme
vmrogue tui --interactive --theme dark
```

## Interactive Features

### 🎯 Dialogs & Confirmations

The interactive TUI shows confirmation dialogs for destructive operations:

- **Start VM**: Press `s` on a stopped VM → Confirmation dialog
- **Stop VM**: Press `x` on a running VM → Confirmation dialog
- **Delete VM**: Press `d` on any VM → Warning confirmation dialog

**Dialog Controls:**
- `←` / `→` / `Tab`: Toggle between Yes/No
- `Enter`: Confirm selection
- `Esc`: Cancel

### 📝 Input Forms

Create resources with interactive forms:

**Create VM** (Press `c` in VM List):
```
┌────── Create Virtual Machine ──────┐
│ VM Name:                           │
│   my-vm█                           │
│                                    │
│ Template:                          │
│   ubuntu-22.04                     │
│                                    │
│ Profile:                           │
│   dev                              │
│                                    │
│ Disk Size:                         │
│   20Gi                             │
│                                    │
│  [ Submit ]  [ Cancel ]            │
│                                    │
│ Tab: Next field | Enter: Submit    │
└────────────────────────────────────┘
```

**Form Controls:**
- `Tab`: Move to next field
- `Shift+Tab`: Move to previous field
- Type to edit current field
- `Backspace`: Delete character
- `Enter`: Submit form
- `Esc`: Cancel

**Create Snapshot** (Press `c` in Snapshots):
- Auto-fills current VM name
- Prompts for snapshot name
- Creates snapshot on submit

### 📋 Context Menus

Right-click style menus with keyboard navigation (Press `m` in VM List):

```
┌────── VM: my-ubuntu ──────┐
│ ▶ Start (s)               │
│   Delete (d)              │
│   Create Snapshot (c)     │
│   View Details (v)        │
│   Edit (e) [disabled]     │
└───────────────────────────┘
```

**Menu Controls:**
- `↑` / `↓` / `j` / `k`: Navigate items
- `Enter`: Select highlighted item
- Type the key letter (e.g., `s` for Start)
- `Esc`: Close menu

**Dynamic Menus:**
- Running VMs show: Stop, Restart, Create Snapshot
- Stopped VMs show: Start, Delete
- Disabled items shown in gray

### 🔔 Toast Notifications

Non-intrusive notifications appear in the top-right corner:

**Types:**
- **Info** (Blue ℹ): "Refreshing data..."
- **Success** (Green ✓): "VM 'my-vm' started successfully"
- **Warning** (Yellow ⚠): "VM may take a few minutes to start"
- **Error** (Red ✗): "Failed to connect to cluster"

**Features:**
- Auto-dismiss after 3 seconds
- Stack multiple notifications
- Don't block user interaction

### 🔍 Search & Filter

Press `/` to activate search bar:

```
┌────────────────────────────┐
│ Search: ubuntu█            │
└────────────────────────────┘
```

**Search Controls:**
- `/`: Open search bar
- Type to filter
- `Enter` or `Esc`: Close search
- Filters VM list in real-time

### ⏳ Progress Indicators

Long-running operations show progress:

```
┌─────── Creating VM ───────┐
│                           │
│  Creating VM 'web-01'...  │
│                           │
│  ████████░░░░░░░░░  45%   │
│                           │
└───────────────────────────┘
```

**Types:**
- **Determinate**: Shows percentage (VM creation, snapshot)
- **Indeterminate**: Spinner for unknown duration (waiting for status)

## View Navigation

### Global Keybindings

| Key | Action |
|-----|--------|
| `1` | Dashboard |
| `2` | VM List |
| `3` | Snapshots |
| `4` | Profiles |
| `5` | Blueprints |
| `?` | Help Screen |
| `Ctrl+R` | Refresh Data |
| `/` | Search/Filter |
| `q` or `Esc` | Quit |

### VM List View

| Key | Action |
|-----|--------|
| `↑↓` or `j/k` | Navigate VMs |
| `Enter` | View details |
| `m` | Open context menu |
| `c` | Create new VM |
| `s` | Start selected VM |
| `x` | Stop selected VM |
| `d` | Delete selected VM |

### Snapshots View

| Key | Action |
|-----|--------|
| `↑↓` or `j/k` | Navigate snapshots |
| `c` | Create snapshot |
| `d` | Delete snapshot |
| `r` | Restore snapshot |

### VM Details View

| Key | Action |
|-----|--------|
| `Backspace` or `Esc` | Return to VM list |

## Interactive Workflows

### Creating a VM

1. Press `2` → Navigate to VM List
2. Press `c` → Open "Create VM" form
3. Fill in fields:
   - VM Name: `web-server`
   - Template: `ubuntu-22.04`
   - Profile: `web`
   - Disk Size: `40Gi`
4. Press `Enter` → Submit
5. Toast notification: "Creating VM 'web-server'..."
6. Progress indicator shows creation status
7. Success notification: "VM 'web-server' created"
8. Auto-refresh shows new VM in list

### Managing a VM

1. Navigate to VM in list (↑↓)
2. Press `m` → Open context menu
3. Select action:
   - `s` → Start VM (with confirmation)
   - `x` → Stop VM (with confirmation)
   - `c` → Create snapshot
   - `v` → View details
4. Confirm action in dialog
5. Real-time notification of status
6. VM list auto-updates

### Taking a Snapshot

1. Navigate to Snapshots view (`3`)
2. Press `c` → Open "Create Snapshot" form
3. VM name auto-filled
4. Enter snapshot name: `backup-2026-02-11`
5. Press `Enter` → Submit
6. Progress bar shows snapshot creation
7. Success notification
8. Snapshot appears in list

## Customization

### Themes

```bash
# Dark theme (default)
vmrogue tui --interactive --theme dark

# Light theme
vmrogue tui --interactive --theme light
```

### Configuration

Edit `~/.config/veyron/tui.toml`:

```toml
[theme]
name = "dark"

[ui]
auto_refresh_interval = 5
show_timestamps = true

[behavior]
confirm_stop = true
confirm_delete = true
```

## Advanced Features

### Multi-Select Operations (Coming Soon)
- Select multiple VMs with `Space`
- Batch operations with confirmation
- Progress for multiple VMs

### Custom Keybindings (Configurable)
```toml
[keybindings]
quit = "q"
refresh = "r"
search = "/"
create = "c"
```

### Mouse Support (Optional)
- Click to select items
- Click buttons in dialogs
- Scroll with mouse wheel

## Tips & Tricks

1. **Quick Actions**: Use single-key shortcuts (`s`, `x`, `d`) instead of menu for speed
2. **Search Everything**: Press `/` to filter large VM lists quickly
3. **Keyboard Only**: Entire TUI navigable without mouse
4. **Confirmation Dialogs**: Default selection is "No" for safety
5. **Auto-Refresh**: Data refreshes every 5 seconds automatically
6. **Stack Notifications**: Multiple actions show stacked notifications
7. **Escape Hatch**: `Esc` always closes dialogs/menus/search

## Troubleshooting

**Dialog not appearing?**
- Ensure terminal is large enough (minimum 80x24)
- Try maximizing terminal window

**Notifications disappearing too fast?**
- Edit config: `notification_duration = 5` (seconds)

**Colors look wrong?**
- Check terminal supports 256 colors
- Try different theme: `--theme light`

**Keyboard shortcuts not working?**
- Check if terminal emulator intercepts keys
- Some keys may conflict with terminal shortcuts

## Comparison: Basic vs Interactive Mode

| Feature | Basic Mode | Interactive Mode |
|---------|-----------|------------------|
| View navigation | ✓ | ✓ |
| List navigation | ✓ | ✓ |
| Confirmation dialogs | ✗ | ✓ |
| Input forms | ✗ | ✓ |
| Context menus | ✗ | ✓ |
| Toast notifications | ✗ | ✓ |
| Progress indicators | ✗ | ✓ |
| Search/filter | ✗ | ✓ |
| Real-time updates | ✓ | ✓ |

## Examples

### Complete Workflow: Deploy a Web Application

```bash
# Launch interactive TUI
vmrogue tui --interactive

# Create database VM
1. Press 2 (VM List)
2. Press c (Create)
3. Fill: name=db, template=postgres, profile=database
4. Submit

# Create web server VMs
5. Press c (Create)
6. Fill: name=web-01, template=ubuntu-22.04, profile=web
7. Submit
8. Repeat for web-02, web-03

# Verify all running
9. See all VMs in list with green "Running" status

# Take snapshots before deployment
10. Press 3 (Snapshots)
11. For each VM, press c, name snapshot "pre-deploy"
12. Notifications confirm snapshots created

# Monitor in dashboard
13. Press 1 (Dashboard)
14. View: 4 Running, 0 Stopped
```

## Keyboard Shortcuts Reference Card

```
┌─────────────────────────────────────────┐
│         VMRogue TUI Shortcuts         │
├─────────────────────────────────────────┤
│ NAVIGATION                              │
│  1-5        Switch views                │
│  ↑↓ / jk    Navigate lists              │
│  Enter      Select / Details            │
│  Backspace  Go back                     │
│                                         │
│ ACTIONS                                 │
│  c          Create new                  │
│  s          Start / Stop                │
│  d          Delete                      │
│  m          Context menu                │
│                                         │
│ GLOBAL                                  │
│  /          Search                      │
│  Ctrl+R     Refresh                     │
│  ?          Help                        │
│  q / Esc    Quit / Cancel               │
└─────────────────────────────────────────┘
```

Save this card for quick reference!
