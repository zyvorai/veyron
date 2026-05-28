# VMRogue Interactive TUI - Complete Feature Demo

**See also:** [Documentation index](README.md) for related guides.

## 🎨 Design Overview

The VMRogue Interactive TUI is a modern, user-friendly terminal interface with:
- **Dialogs & Confirmations** - Modal dialogs for safe operations
- **Input Forms** - Multi-field forms for resource creation
- **Context Menus** - Right-click style action menus
- **Toast Notifications** - Non-blocking status messages
- **Progress Indicators** - Visual feedback for long operations
- **Search & Filter** - Real-time list filtering
- **Real-time Updates** - Auto-refresh with visual indicators

## 🚀 Quick Start

```bash
# Basic TUI (view-only)
vmrogue tui

# Full Interactive Mode (recommended)
vmrogue tui --interactive

# With options
vmrogue tui --interactive --theme dark --namespace production
```

## 📋 Widget Showcase

### 1. Confirmation Dialogs

**Purpose**: Prevent accidental destructive actions

**Example: Deleting a VM**
```
┌────────────── Delete VM ──────────────┐
│                                       │
│  ⚠ Permanently delete VM 'web-01'?   │
│     This cannot be undone!            │
│                                       │
│         [ No ]     [ Yes ]            │
│                                       │
└───────────────────────────────────────┘
```

**Features:**
- Default selection is "No" for safety
- Toggle with `←` `→` `Tab`
- Color-coded (Red=No, Green=Yes)
- Keyboard shortcuts work

**Dialog Types:**
- ✓ Success (Green border)
- ⚠ Warning (Yellow border)
- ✗ Error (Red border)
- ℹ Info (Cyan border)

### 2. Input Forms

**Purpose**: Create resources with guided input

**Example: Create VM Form**
```
┌────────── Create Virtual Machine ──────────┐
│                                             │
│ VM Name:                                    │
│   web-server-01█                            │
│                                             │
│ Template:                                   │
│   ubuntu-22.04                              │
│                                             │
│ Profile:                                    │
│   web                                       │
│                                             │
│ Disk Size:                                  │
│   40Gi                                      │
│                                             │
│        [ Submit ]    [ Cancel ]             │
│                                             │
│ Tab: Next field | Enter: Submit | Esc: Cancel │
└─────────────────────────────────────────────┘
```

**Features:**
- Tab navigation between fields
- Active field highlighted in yellow
- Placeholders for empty fields
- Visual cursor (█) shows typing position
- Validation on submit
- Escape to cancel

**Use Cases:**
- Create VM (`c` in VM List)
- Create Snapshot (`c` in Snapshots)
- Edit Configuration (future)
- Batch Operations (interactive TUI: multi-select + bulk start/stop/delete)

### 3. Context Menus

**Purpose**: Contextual actions for selected resources

**Example: VM Actions Menu**
```
┌────── VM: web-server-01 ──────┐
│ ▶ Stop (s)                    │
│   Restart (r)                 │
│   Create Snapshot (c)         │
│   Console (o) [disabled]      │
│   View Details (v)            │
│   Edit (e) [disabled]         │
└───────────────────────────────┘
```

**Features:**
- Contextual items based on VM state
- Disabled items shown in gray
- Arrow (▶) shows selection
- Quick access via single key
- Position near selected item

**Dynamic Behavior:**
- Running VM: Stop, Restart, Snapshot
- Stopped VM: Start, Delete
- Pending VM: Only View Details

### 4. Toast Notifications

**Purpose**: Non-blocking status updates

**Example: Notification Stack**
```
Top-right corner:
┌─────────────────────────────────────┐
│ ✓ VM 'web-01' started successfully  │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│ ℹ Refreshing VM list...             │
└─────────────────────────────────────┘
```

**Features:**
- Auto-dismiss after 3 seconds
- Stack multiple notifications
- Color-coded by type
- Icon indicators
- Don't block interaction

**Notification Types:**
| Icon | Type | Color | Example |
|------|------|-------|---------|
| ℹ | Info | Cyan | "Refreshing data..." |
| ✓ | Success | Green | "VM created successfully" |
| ⚠ | Warning | Yellow | "VM may take time to start" |
| ✗ | Error | Red | "Connection failed" |

### 5. Progress Indicators

**Purpose**: Visual feedback for long operations

**Example: Determinate Progress**
```
┌──────── Creating VM ────────┐
│                             │
│  Creating VM 'db-01'...     │
│                             │
│  ████████████░░░░░░  65%    │
│                             │
└─────────────────────────────┘
```

**Example: Indeterminate Progress**
```
┌──────── Please Wait ────────┐
│                             │
│  Connecting to cluster...   │
│                             │
│       ⠋ Working...          │
│                             │
└─────────────────────────────┘
```

**Features:**
- Percentage for known duration
- Spinner for unknown duration
- Descriptive messages
- Cancel with `Esc` (if supported)

### 6. Search Bar

**Purpose**: Filter large lists in real-time

**Example: Search Active**
```
Bottom of screen:
┌────────────────────────────┐
│ Search: web█               │
└────────────────────────────┘

VM List shows only:
- web-server-01
- web-server-02
- web-db-backend
```

**Features:**
- Activate with `/`
- Type to filter
- Real-time results
- Case-insensitive
- `Enter` or `Esc` to close

## 🎯 Complete Interactive Workflows

### Workflow 1: Deploy a New Application

**Goal**: Create and start a new VM

```
Steps:
1. Launch TUI
   $ vmrogue tui --interactive

2. Navigate to VM List
   Press: 2

3. Open Create Dialog
   Press: c

4. Fill Form
   VM Name: my-app
   Template: ubuntu-22.04
   Profile: web
   Disk Size: 30Gi
   Press: Enter

5. Monitor Creation
   Notification: ℹ Creating VM 'my-app'...
   Progress bar: 0% → 100%
   Notification: ✓ VM 'my-app' created

6. Start VM
   Select VM (arrows)
   Press: s
   Confirm: Tab → Enter
   Notification: ✓ VM started

7. Verify Status
   List shows: Green "Running" status
```

### Workflow 2: Safe VM Deletion

**Goal**: Delete a VM with confirmation

```
Steps:
1. Select VM in List
   Use ↑↓ to highlight VM

2. Trigger Delete
   Press: d

3. Confirmation Dialog Appears
   ┌────── Delete VM ──────┐
   │ Delete VM 'old-vm'?   │
   │   [ No ]  [ Yes ]     │
   └───────────────────────┘
   Default: No (highlighted)

4. Review & Confirm
   Read message carefully
   Press: Tab (to select Yes)
   Press: Enter (to confirm)

5. Deletion in Progress
   Notification: ℹ Deleting VM...
   Progress indicator

6. Success
   Notification: ✓ VM deleted
   VM removed from list
```

### Workflow 3: Batch Snapshot Creation

**Goal**: Create snapshots for multiple VMs

```
Steps:
1. Navigate to Snapshots
   Press: 3

2. Create First Snapshot
   Press: c
   Fill: VM=web-01, Name=backup-20260211
   Submit: Enter
   Notification: ✓ Snapshot created

3. Repeat for Other VMs
   Press: c
   Fill: VM=web-02, Name=backup-20260211
   Submit: Enter

4. Verify All Snapshots
   List shows all 3 snapshots
   Status: Green "Ready"

5. Monitor in Dashboard
   Press: 1
   See snapshot count updated
```

### Workflow 4: Context Menu Navigation

**Goal**: Use context menu for VM operations

```
Steps:
1. Select VM
   Navigate to VM in list

2. Open Context Menu
   Press: m

3. Menu Appears at Selection
   ┌──── VM: web-01 ────┐
   │ ▶ Stop (s)         │
   │   Snapshot (c)     │
   │   Details (v)      │
   └────────────────────┘

4. Navigate Menu
   ↑↓ or j/k to move
   Or press 's' directly

5. Execute Action
   Press: Enter
   Confirmation dialog appears

6. Confirm
   Dialog for chosen action
   Complete operation
```

## 🎨 Visual Design Principles

### Color Coding
- **Cyan**: Headers, titles, info
- **Green**: Success, running status
- **Red**: Errors, stop, delete
- **Yellow**: Warnings, input focus
- **Gray**: Disabled items, muted text

### Layout Structure
```
┌──────────────────────────────────────┐
│            Header (Cyan)             │ ← View title
├──────────────────────────────────────┤
│                                      │
│         Main Content Area            │ ← Tables, details, etc.
│                                      │
├──────────────────────────────────────┤
│         Help Bar (Gray)              │ ← Keyboard shortcuts
└──────────────────────────────────────┘

Overlays:
- Dialogs: Centered modal
- Menus: Near selection
- Notifications: Top-right stack
- Search: Bottom center
```

### Accessibility
- No mouse required
- Clear keyboard shortcuts
- High contrast colors
- Visual cursor indicators
- Descriptive labels

## 🔧 Configuration

### Config File: `~/.config/vmrogue/tui.toml`

```toml
[theme]
name = "dark"  # or "light"

[ui]
auto_refresh_interval = 5  # seconds
show_notifications = true
notification_duration = 3  # seconds

[behavior]
confirm_stop = true
confirm_delete = true
confirm_start = false  # Skips start confirmations

[keybindings]
quit = "q"
search = "/"
refresh = "Ctrl+R"
create = "c"
context_menu = "m"
```

### Theme Customization

**Dark Theme** (Default):
- Background: Black
- Text: White
- Borders: Cyan
- Highlights: Yellow
- Success: Green
- Error: Red

**Light Theme**:
- Background: White
- Text: Black
- Borders: Blue
- Highlights: Orange
- Success: Dark Green
- Error: Dark Red

## 📊 Feature Comparison Matrix

| Feature | Basic TUI | Interactive TUI |
|---------|-----------|-----------------|
| View navigation | ✓ | ✓ |
| List browsing | ✓ | ✓ |
| Auto-refresh | ✓ | ✓ |
| Real-time updates | ✓ | ✓ |
| **Confirmation dialogs** | ✗ | ✓ |
| **Input forms** | ✗ | ✓ |
| **Context menus** | ✗ | ✓ |
| **Toast notifications** | ✗ | ✓ |
| **Progress indicators** | ✗ | ✓ |
| **Search/filter** | ✗ | ✓ |
| **Error recovery** | ✗ | ✓ |
| **Batch operations** | ✓ (interactive TUI) | ✓ |
| **Mouse support** | ✗ | ✓ (scroll + click VM list) |

## 🎓 Tips & Best Practices

### Keyboard Efficiency
1. **Learn single-key shortcuts**: `s`, `x`, `d`, `c` are faster than menus
2. **Use `/` for search**: Filter large lists quickly
3. **Master view navigation**: `1-5` to jump between views
4. **Tab through forms**: Don't use arrow keys in dialogs

### Safety Tips
1. **Read confirmations**: Default is always "No"
2. **Check VM status**: Color-coded for quick recognition
3. **Monitor notifications**: Success/error feedback
4. **Use snapshots before risky operations**: Press `c` in Snapshots view

### Performance Tips
1. **Auto-refresh is smart**: Only refreshes when idle
2. **Search doesn't trigger API calls**: Local filtering only
3. **Notifications auto-clear**: Don't accumulate
4. **Progress bars don't block**: Can cancel if needed

## 🐛 Troubleshooting

**Problem: Dialogs not showing**
- Solution: Terminal too small. Resize to 80x24 minimum

**Problem: Colors look weird**
- Solution: Terminal doesn't support 256 colors. Try `--theme light`

**Problem: Notifications disappear too fast**
- Solution: Edit config `notification_duration = 5`

**Problem: Can't type in input fields**
- Solution: Make sure dialog is focused. Press `Esc` to reset

**Problem: Keyboard shortcuts conflict**
- Solution: Check terminal shortcuts. Customize in config

## 🚀 Future Enhancements

### Planned Features
- [x] Multi-select with checkboxes
- [x] Batch operations (start/stop/delete multiple VMs in interactive TUI)
- [x] Mouse support (scroll lists; left-click selects VM row)
- [x] Export operations (CSV) — press `E` on VM list (writes `~/.local/share/vmrogue/vm-export-*.csv`)
- [ ] Custom themes (user-defined colors)
- [ ] Saved filters and searches
- [ ] VM console integration
- [ ] Log viewer in details view
- [ ] Resource graphs (CPU, memory over time)
- [ ] Export operations (CSV, JSON)
- [ ] Macro recording (repeat actions)

### Community Requests
- Fuzzy search (not just substring)
- Vim-style command mode (`:delete vm-name`)
- Split-pane view (multiple views at once)
- Customizable dashboard widgets
- Notification history panel

## 📚 Related Documentation

- [User Guide](./USER_GUIDE.md) - Complete CLI reference
- [Interactive TUI Guide](./INTERACTIVE_TUI.md) - Detailed keyboard shortcuts
- [Configuration](./CONFIGURATION.md) - Config file options
- [Troubleshooting](./TROUBLESHOOTING.md) - Common issues

## 🎬 Video Demo (Coming Soon)

Watch a video walkthrough of all interactive features:
- Creating VMs with forms
- Using context menus
- Managing snapshots
- Real-time monitoring
- Keyboard navigation tips

---

**Ready to try?**

```bash
vmrogue tui --interactive
```

Press `?` for help anytime!
