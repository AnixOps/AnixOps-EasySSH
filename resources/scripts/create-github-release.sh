#!/bin/bash
# Create GitHub release for EasySSH

set -e

VERSION="0.3.0"
TAG="v${VERSION}"
RELEASE_DIR="release"

echo "=== Creating GitHub Release ${TAG} ==="
echo ""

# Check if tag already exists
if git rev-parse "${TAG}" >/dev/null 2>&1; then
    echo "Tag ${TAG} already exists"
else
    echo "Creating tag ${TAG}..."
    git tag -a "${TAG}" -m "Release ${TAG} - Native Foundations"
    git push origin "${TAG}"
fi

# Create release using gh CLI (if available)
if command -v gh &> /dev/null; then
    echo "Creating GitHub release..."
    gh release create "${TAG}" \
        --title "EasySSH ${TAG} - Native Foundations" \
        --notes-file RELEASE_NOTES.md \
        --draft \
        ${RELEASE_DIR}/EasySSH-v${VERSION}-x86_64.exe \
        ${RELEASE_DIR}/sha256sums.txt \
        ${RELEASE_DIR}/*.sha256
else
    echo "GitHub CLI (gh) not found. Please create release manually:"
    echo "  https://github.com/anixops/easyssh/releases/new?tag=${TAG}"
fi

echo ""
echo "=== Release Created ==="
echo "Review the draft release on GitHub and publish when ready."
