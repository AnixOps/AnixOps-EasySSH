#!/bin/bash
# Tag and prepare GitHub release for EasySSH v0.3.0

set -e

VERSION="0.3.0"
TAG="v${VERSION}"
DATE=$(date +%Y-%m-%d)

echo "=== EasySSH GitHub Release Preparation ==="
echo "Version: ${VERSION}"
echo "Tag: ${TAG}"
echo "Date: ${DATE}"
echo ""

# Check if tag exists
echo "Checking git tag..."
if git rev-parse "${TAG}" >/dev/null 2>&1; then
    echo "Tag ${TAG} already exists"
    read -p "Delete and recreate? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git tag -d "${TAG}"
        git push origin ":refs/tags/${TAG}"
        git tag -a "${TAG}" -m "EasySSH ${TAG} - Native Foundations"
        git push origin "${TAG}"
    fi
else
    echo "Creating tag ${TAG}..."
    git tag -a "${TAG}" -m "EasySSH ${TAG} - Native Foundations"
    git push origin "${TAG}"
fi

echo ""
echo "=== Release Commands ==="
echo ""
echo "To create the GitHub release (run these commands):"
echo ""
echo "1. Create draft release:"
echo "   gh release create ${TAG} \\"
echo "     --title \"EasySSH ${TAG} - Native Foundations\" \\"
echo "     --notes-file RELEASE_NOTES.md \\"
echo "     --draft \\"
echo "     EasySSH-v${VERSION}-x86_64.exe \\"
echo "     SHA256SUMS.txt"
echo ""
echo "2. Or manually at:"
echo "   https://github.com/anixops/easyssh/releases/new?tag=${TAG}"
echo ""
echo "=== Version Tag Summary ==="
echo "Tag: ${TAG}"
echo "Commit: $(git rev-parse HEAD)"
echo "Branch: $(git branch --show-current)"
echo ""
