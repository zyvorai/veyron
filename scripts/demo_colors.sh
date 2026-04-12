#!/bin/bash
# Demo script to show VMRogue themed CLI output

echo "🎨 VMRogue CLI Theme Demo"
echo ""
echo "This demo shows the themed CLI output for various commands."
echo "Note: Some commands require a Kubernetes cluster with VMs."
echo ""

# Show available templates with colors
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1. Templates (shows OS-specific colors)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
./target/release/vmrogue templates
echo ""

# Show validation message
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "2. Validation (shows success message)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ -f "examples/basic-vm.yaml" ]; then
    ./target/release/vmrogue validate examples/basic-vm.yaml
else
    echo "  (No example file found - would show: ✓ Configuration is valid)"
fi
echo ""

# Show help to display all colors
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "3. Help Menu"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
./target/release/vmrogue --help
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "To see more themed output, try:"
echo "  • vmrogue list             (requires cluster)"
echo "  • vmrogue get <vm-name>    (requires cluster)"
echo "  • vmrogue resources        (requires cluster)"
echo "  • vmrogue wizard           (interactive)"
echo ""
echo "All success messages (✓) are shown in GREEN"
echo "All error messages (✗) are shown in RED"
echo "All info messages (ℹ) are shown in BLUE"
echo "VM status symbols are color-coded by state"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
