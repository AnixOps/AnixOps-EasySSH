#!/bin/bash
# EasySSH Release Manifest Generator
# Generates JSON manifest with release information and download URLs

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

VERSION="${1:-$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/')}"
RELEASE_DIR="$PROJECT_ROOT/releases"
OUTPUT_FILE="${2:-$RELEASE_DIR/release-manifest-$VERSION.json}"

echo "Generating release manifest for v$VERSION..."

# Detect channel from version
if [[ "$VERSION" == *"alpha"* ]]; then
    CHANNEL="alpha"
elif [[ "$VERSION" == *"beta"* ]]; then
    CHANNEL="beta"
elif [[ "$VERSION" == *"rc"* ]]; then
    CHANNEL="rc"
else
    CHANNEL="stable"
fi

# Generate manifest
cat > "$OUTPUT_FILE" << EOF
{
  "schema_version": "1.0",
  "release": {
    "version": "$VERSION",
    "channel": "$CHANNEL",
    "release_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "eol_date": null,
    "supported": true
  },
  "downloads": {
    "windows": {
      "lite": {
        "x64": {
          "zip": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-lite-$VERSION-windows-x64.zip",
            "checksum_sha256": "${LITE_WIN_X64_SHA256:-PLACEHOLDER}"
          },
          "msi": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-lite-$VERSION-windows-x64.msi",
            "checksum_sha256": "${LITE_WIN_X64_MSI_SHA256:-PLACEHOLDER}"
          }
        },
        "arm64": {
          "zip": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-lite-$VERSION-windows-arm64.zip",
            "checksum_sha256": "${LITE_WIN_ARM64_SHA256:-PLACEHOLDER}"
          }
        }
      },
      "standard": {
        "x64": {
          "zip": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-standard-$VERSION-windows-x64.zip",
            "checksum_sha256": "${STD_WIN_X64_SHA256:-PLACEHOLDER}"
          },
          "msi": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-standard-$VERSION-windows-x64.msi",
            "checksum_sha256": "${STD_WIN_X64_MSI_SHA256:-PLACEHOLDER}"
          }
        }
      }
    },
    "linux": {
      "lite": {
        "x64": {
          "tarball": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-lite-$VERSION-linux-x64.tar.gz",
            "checksum_sha256": "${LITE_LINUX_X64_SHA256:-PLACEHOLDER}"
          }
        },
        "arm64": {
          "tarball": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-lite-$VERSION-linux-arm64.tar.gz",
            "checksum_sha256": "${LITE_LINUX_ARM64_SHA256:-PLACEHOLDER}"
          }
        }
      },
      "standard": {
        "x64": {
          "tarball": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-standard-$VERSION-linux-x64.tar.gz",
            "checksum_sha256": "${STD_LINUX_X64_SHA256:-PLACEHOLDER}"
          }
        },
        "arm64": {
          "tarball": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-standard-$VERSION-linux-arm64.tar.gz",
            "checksum_sha256": "${STD_LINUX_ARM64_SHA256:-PLACEHOLDER}"
          }
        }
      },
      "pro": {
        "x64": {
          "tarball": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-pro-server-$VERSION-linux-x64.tar.gz",
            "checksum_sha256": "${PRO_LINUX_X64_SHA256:-PLACEHOLDER}"
          }
        },
        "arm64": {
          "tarball": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-pro-server-$VERSION-linux-arm64.tar.gz",
            "checksum_sha256": "${PRO_LINUX_ARM64_SHA256:-PLACEHOLDER}"
          }
        }
      }
    },
    "macos": {
      "lite": {
        "universal": {
          "dmg": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-lite-$VERSION-macos-universal.dmg",
            "checksum_sha256": "${LITE_MACOS_SHA256:-PLACEHOLDER}"
          }
        }
      },
      "standard": {
        "universal": {
          "dmg": {
            "url": "https://github.com/anixops/easyssh/releases/download/v$VERSION/easyssh-standard-$VERSION-macos-universal.dmg",
            "checksum_sha256": "${STD_MACOS_SHA256:-PLACEHOLDER}"
          }
        }
      }
    },
    "docker": {
      "pro": {
        "amd64": {
          "image": "anixops/easyssh-pro:$VERSION",
          "latest": "anixops/easyssh-pro:latest"
        },
        "arm64": {
          "image": "anixops/easyssh-pro:$VERSION",
          "latest": "anixops/easyssh-pro:latest"
        }
      },
      "lite": {
        "amd64": {
          "image": "anixops/easyssh-lite:$VERSION"
        }
      }
    }
  },
  "system_requirements": {
    "windows": {
      "os": "Windows 10/11",
      "architecture": ["x64", "arm64"],
      "minimum_version": "10.0.19041"
    },
    "linux": {
      "os": "Linux",
      "architecture": ["x64", "arm64"],
      "dependencies": ["gtk4", "libadwaita-1-0"]
    },
    "macos": {
      "os": "macOS",
      "architecture": ["x64", "arm64"],
      "minimum_version": "13.0"
    }
  },
  "release_notes_url": "https://github.com/anixops/easyssh/releases/tag/v$VERSION",
  "documentation_url": "https://docs.anixops.com/easyssh",
  "support_url": "https://github.com/anixops/easyssh/issues",
  "metadata": {
    "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "generated_by": "release-manifest.sh"
  }
}
EOF

echo "Release manifest generated: $OUTPUT_FILE"

# Update latest symlink
ln -sf "$(basename "$OUTPUT_FILE")" "$RELEASE_DIR/release-manifest-latest.json"

echo "Latest manifest link updated"
