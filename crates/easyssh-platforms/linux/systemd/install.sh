#!/bin/bash
# EasySSH Systemd Service Installation Script
# Run as root for system-wide installation

set -e

PREFIX="${PREFIX:-/usr}"
SERVICE_USER="${SERVICE_USER:-easyssh}"
SERVICE_GROUP="${SERVICE_GROUP:-easyssh}"

echo "=== EasySSH Systemd Service Installer ==="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Warning: Not running as root. Installing to user systemd."
    USER_MODE=1
fi

# Create user/group if not exists
if [ -z "$USER_MODE" ]; then
    if ! id "$SERVICE_USER" &>/dev/null; then
        echo "Creating user: $SERVICE_USER"
        useradd -r -s /bin/false -d /var/lib/easyssh -M "$SERVICE_USER"
    fi

    if ! getent group "$SERVICE_GROUP" &>/dev/null; then
        echo "Creating group: $SERVICE_GROUP"
        groupadd -r "$SERVICE_GROUP"
    fi
fi

# Create directories
echo "Creating directories..."
if [ -z "$USER_MODE" ]; then
    install -d -m 755 /var/lib/easyssh
    install -d -m 755 /var/log/easyssh
    install -d -m 755 /run/easyssh

    chown "$SERVICE_USER:$SERVICE_GROUP" /var/lib/easyssh
    chown "$SERVICE_USER:$SERVICE_GROUP" /var/log/easyssh
    chown "$SERVICE_USER:$SERVICE_GROUP" /run/easyssh
else
    install -d -m 755 "$HOME/.local/share/easyssh"
    install -d -m 755 "$HOME/.local/state/easyssh/log"
fi

# Install binary
echo "Installing binary..."
if [ -f "../easyssh-gtk4/target/release/easyssh" ]; then
    install -m 755 "../easyssh-gtk4/target/release/easyssh" "$PREFIX/bin/easyssh"
elif [ -f "../easyssh-gtk4/target/debug/easyssh" ]; then
    install -m 755 "../easyssh-gtk4/target/debug/easyssh" "$PREFIX/bin/easyssh"
else
    echo "Error: EasySSH binary not found. Please build first:"
    echo "  cd ../easyssh-gtk4 && cargo build --release"
    exit 1
fi

# Install systemd service
echo "Installing systemd service..."
if [ -z "$USER_MODE" ]; then
    install -m 644 easyssh.service /etc/systemd/system/easyssh.service

    # Install D-Bus config
    if [ -d "/usr/share/dbus-1/system.d" ]; then
        install -m 644 com.easyssh.Service.conf /usr/share/dbus-1/system.d/com.easyssh.Service.conf
    fi

    # Install D-Bus service
    if [ -d "/usr/share/dbus-1/system-services" ]; then
        install -m 644 com.easyssh.Service.service /usr/share/dbus-1/system-services/com.easyssh.Service.service
    fi

    # Reload systemd
    systemctl daemon-reload

    echo ""
    echo "=== Installation Complete ==="
    echo ""
    echo "Start service:"
    echo "  systemctl start easyssh"
    echo ""
    echo "Enable auto-start:"
    echo "  systemctl enable easyssh"
    echo ""
    echo "Check status:"
    echo "  systemctl status easyssh"
    echo ""
    echo "View logs:"
    echo "  journalctl -u easyssh -f"
    echo ""
else
    # User mode installation
    install -d -m 755 "$HOME/.config/systemd/user"
    sed "s|/var/lib/easyssh|$HOME/.local/share/easyssh|g; \
         s|/var/log/easyssh|$HOME/.local/state/easyssh/log|g; \
         s|/run/easyssh|$HOME/.local/run/easyssh|g; \
         s|ProtectSystem=strict|# ProtectSystem=strict|g; \
         s|ProtectHome=true|# ProtectHome=true|g" \
        easyssh.service > "$HOME/.config/systemd/user/easyssh.service"

    install -d -m 755 "$HOME/.local/run/easyssh"

    systemctl --user daemon-reload

    echo ""
    echo "=== User Installation Complete ==="
    echo ""
    echo "Start service:"
    echo "  systemctl --user start easyssh"
    echo ""
    echo "Enable auto-start:"
    echo "  systemctl --user enable easyssh"
    echo ""
    echo "Check status:"
    echo "  systemctl --user status easyssh"
    echo ""
    echo "View logs:"
    echo "  journalctl --user -u easyssh -f"
    echo ""
fi

echo "Configuration directory:"
if [ -z "$USER_MODE" ]; then
    echo "  /var/lib/easyssh"
else
    echo "  $HOME/.local/share/easyssh"
fi
echo ""
