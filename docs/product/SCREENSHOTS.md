# Screenshot & GIF Guidelines

## Overview

This document provides guidelines for creating screenshots and GIFs for EasySSH documentation.

## Screenshot Requirements

### Dimensions

| Type | Resolution | Format |
|------|------------|--------|
| Hero/Banner | 1920x1080 | PNG |
| Feature Screenshots | 1440x900 | PNG |
| UI Elements | 800x600 | PNG |
| Mobile | 390x844 | PNG |

### Platforms to Capture

1. **macOS**
   - Light mode
   - Dark mode
   - Different terminal themes (Dracula, Solarized, etc.)

2. **Windows**
   - Windows 10
   - Windows 11
   - Windows Terminal vs CMD

3. **Linux**
   - GNOME (Ubuntu)
   - KDE (Fedora)
   - Different distributions

## Screenshot Checklist

### Lite Edition

- [ ] Main window with server list
- [ ] Add server dialog
- [ ] Search in action
- [ ] Native terminal being launched
- [ ] Settings/preferences
- [ ] Import dialog
- [ ] Dark mode version

### Standard Edition

- [ ] Main workspace with sidebar
- [ ] Terminal tabs
- [ ] Split panes (2x2, triple, custom)
- [ ] SFTP panel
- [ ] Command palette
- [ ] Settings with terminal options
- [ ] Multiple themes
- [ ] Keyboard shortcuts

### Pro Edition

- [ ] Admin dashboard
- [ ] Team management
- [ ] RBAC configuration
- [ ] Audit logs
- [ ] SSO setup
- [ ] Server sharing
- [ ] Snippets management

## GIF Requirements

### Specifications

| Parameter | Value |
|-----------|-------|
| Resolution | 800x600 or 1200x800 |
| Frame Rate | 15-30 fps |
| Duration | 5-15 seconds |
| Format | GIF or WebM |
| Max Size | 5MB |

### GIF Scenarios

#### Lite

1. **Quick Connect** (5s)
   - Click server
   - Terminal opens
   - Connected

2. **Search** (5s)
   - Press Cmd+K
   - Type "prod"
   - Filter results

3. **Add Server** (8s)
   - Click +
   - Fill form
   - Test connection
   - Save

#### Standard

1. **Terminal Tabs** (8s)
   - Open multiple tabs
   - Switch between them
   - Close one

2. **Split Panes** (10s)
   - Create vertical split
   - Create horizontal split
   - Resize panes
   - Close one

3. **SFTP Transfer** (10s)
   - Open SFTP panel
   - Drag file to upload
   - Right-click to download

#### Pro

1. **Invite Team Member** (10s)
   - Open team settings
   - Click invite
   - Enter email
   - Send invitation

2. **View Audit Log** (8s)
   - Open audit section
   - Apply filters
   - Export log

## Tools

### macOS

- **CleanShot X** - Best for screenshots
- **ScreenFlow** - For GIFs/videos
- **Kap** - Free GIF recorder

### Windows

- **ShareX** - Screenshots and GIFs
- **ScreenToGif** - GIF creation
- **OBS Studio** - Video recording

### Linux

- **Flameshot** - Screenshots
- **Peek** - GIF recording
- **OBS Studio** - Video

## Naming Convention

```
{platform}-{edition}-{feature}-{theme}-{size}.{ext}

Examples:
- macos-lite-main-window-light-1440x900.png
- windows-standard-split-panes-dark-1440x900.png
- linux-pro-audit-logs-1440x900.png
- all-lite-quick-connect-800x600.gif
```

## Storage Location

```
docs-product/
└── public/
    └── images/
        ├── screenshots/
        │   ├── macos/
        │   ├── windows/
        │   └── linux/
        └── gifs/
            ├── lite/
            ├── standard/
            └── pro/
```

## Accessibility

- Include alt text for all images
- Ensure sufficient contrast
- Provide captions for GIFs
- Consider color blindness

## Current Status

### Screenshots to Capture

| Edition | Platform | Status |
|---------|----------|--------|
| Lite | macOS | 🔄 In Progress |
| Lite | Windows | ⏳ Pending |
| Lite | Linux | ⏳ Pending |
| Standard | macOS | ⏳ Pending |
| Standard | Windows | ⏳ Pending |
| Standard | Linux | ⏳ Pending |
| Pro | Web | ⏳ Pending |

### GIFs to Create

| Feature | Edition | Status |
|---------|---------|--------|
| Quick Connect | Lite | ⏳ Pending |
| Search | Lite | ⏳ Pending |
| Terminal Tabs | Standard | ⏳ Pending |
| Split Panes | Standard | ⏳ Pending |
| SFTP | Standard | ⏳ Pending |
| Team Invite | Pro | ⏳ Pending |

Legend:
- ✅ Complete
- 🔄 In Progress
- ⏳ Pending

## Contribution

To contribute screenshots or GIFs:

1. Follow the specifications above
2. Use consistent styling
3. Submit via PR to `docs-product/public/images/`
4. Include description of what's shown
5. Ensure no sensitive data is visible

## Contact

For questions about screenshots/GIFs:
- docs@easyssh.dev
- Discord: #documentation channel
