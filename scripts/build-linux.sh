#!/bin/bash
# EasySSH Linux Build Script
# Run this on a Linux machine with GTK4 and libadwaita installed

set -e

VERSION="0.3.0"
echo "Building EasySSH v${VERSION} for Linux..."

# Install dependencies if needed
# sudo apt-get update
# sudo apt-get install -y libgtk-4-dev libadwaita-1-dev pkg-config

cd platforms/linux/easyssh-gtk4

# Build release
cargo build --release

cd ../../..

# Create package
mkdir -p "releases/v${VERSION}/linux/easyssh-${VERSION}-linux-x64/usr/bin"
mkdir -p "releases/v${VERSION}/linux/easyssh-${VERSION}-linux-x64/usr/share/applications"

cp target/release/easyssh "releases/v${VERSION}/linux/easyssh-${VERSION}-linux-x64/usr/bin/"

# Create desktop entry
cat > "releases/v${VERSION}/linux/easyssh-${VERSION}-linux-x64/usr/share/applications/easyssh.desktop" << EOF
[Desktop Entry]
Name=EasySSH
Comment=Native SSH Client
Exec=/usr/bin/easyssh
Icon=easyssh
Type=Application
Categories=Network;RemoteAccess;
Terminal=false
Version=${VERSION}
EOF

# Create install script
cat > "releases/v${VERSION}/linux/easyssh-${VERSION}-linux-x64/install.sh" << 'EOF'
#!/bin/bash
set -e
echo "Installing EasySSH..."
sudo cp usr/bin/easyssh /usr/local/bin/
sudo chmod +x /usr/local/bin/easyssh
sudo cp usr/share/applications/easyssh.desktop /usr/share/applications/
echo "EasySSH installed!"
EOF
chmod +x "releases/v${VERSION}/linux/easyssh-${VERSION}-linux-x64/install.sh"

# Create tarball
cd "releases/v${VERSION}/linux"
tar -czf "easyssh-${VERSION}-linux-x64.tar.gz" "easyssh-${VERSION}-linux-x64"

echo "Linux build complete: easyssh-${VERSION}-linux-x64.tar.gz"
