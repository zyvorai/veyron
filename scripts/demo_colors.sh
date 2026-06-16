#!/bin/bash
# Demo script to show Veyron themed CLI output

echo "🎨 Veyron CLI Theme Demo"
echo ""
echo "This demo shows the themed CLI output for various commands."
echo "Note: Some commands require a Kubernetes cluster with VMs."
echo ""

# Show available templates with colors
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1. Templates (shows OS-specific colors)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
./target/release/veyron templates
echo ""

# Show validation message
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "2. Validation (shows success message)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ -f "examples/basic-vm.yaml" ]; then
    ./target/release/veyron validate examples/basic-vm.yaml
else
    echo "  (No example file found - would show: ✓ Configuration is valid)"
fi
echo ""

# Show help to display all colors
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "3. Help Menu"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
./target/release/veyron --help
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "To see more themed output, try:"
echo "  • veyron list             (requires cluster)"
echo "  • veyron get <vm-name>    (requires cluster)"
echo "  • veyron resources        (requires cluster)"
echo "  • veyron wizard           (interactive)"
echo ""
echo "All success messages (✓) are shown in GREEN"
echo "All error messages (✗) are shown in RED"
echo "All info messages (ℹ) are shown in BLUE"
echo "VM status symbols are color-coded by state"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
