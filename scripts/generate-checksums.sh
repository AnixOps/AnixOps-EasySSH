#!/bin/bash
# Generate checksums for release files

set -e

VERSION="0.3.0"
RELEASE_DIR="release"
CHECKSUM_FILE="${RELEASE_DIR}/sha256sums.txt"

echo "=== Generating Checksums for EasySSH v${VERSION} ==="
echo ""

cd ${RELEASE_DIR}

# Generate SHA-256 checksums for all files
echo "Generating SHA-256 checksums..."
sha256sum * > sha256sums.txt

echo ""
echo "=== Checksums Generated ==="
cat sha256sums.txt

echo ""
echo "=== Individual .sha256 Files ==="
# Create individual checksum files for each asset
for file in *; do
    if [ -f "$file" ] && [ "$file" != "sha256sums.txt" ]; then
        sha256sum "$file" > "${file}.sha256"
        echo "Created ${file}.sha256"
    fi
done

echo ""
echo "=== All Files in Release Directory ==="
ls -lh
