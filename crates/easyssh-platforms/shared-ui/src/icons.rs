//! Icon System
//!
//! Cross-platform icon system providing:
//! - Unified icon API for all platforms
//! - Icon sets (Lucide, SF Symbols, Material)
//! - Dynamic icon sizing
//! - Platform-native icon fallbacks
//! - Accessibility labels

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Icon size presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IconSize {
    /// Extra small (12px)
    Xs,
    /// Small (16px)
    Small,
    /// Medium (20px) - default
    #[default]
    Medium,
    /// Large (24px)
    Large,
    /// Extra large (32px)
    Xl,
    /// 2x extra large (48px)
    Xxl,
}

impl IconSize {
    /// Get pixel size
    pub fn as_px(&self) -> u32 {
        match self {
            IconSize::Xs => 12,
            IconSize::Small => 16,
            IconSize::Medium => 20,
            IconSize::Large => 24,
            IconSize::Xl => 32,
            IconSize::Xxl => 48,
        }
    }

    /// Get size as string
    pub fn as_str(&self) -> &'static str {
        match self {
            IconSize::Xs => "12",
            IconSize::Small => "16",
            IconSize::Medium => "20",
            IconSize::Large => "24",
            IconSize::Xl => "32",
            IconSize::Xxl => "48",
        }
    }
}

/// Icon sets supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IconSet {
    /// Lucide icons (default, cross-platform)
    #[default]
    Lucide,
    /// SF Symbols (Apple platforms only)
    SfSymbols,
    /// Material Design icons
    Material,
    /// Custom icon set
    Custom,
}

/// Icon definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Icon {
    /// Icon identifier
    pub id: IconId,
    /// Icon set
    pub set: IconSet,
    /// Size
    pub size: IconSize,
    /// Color (optional, defaults to current)
    pub color: Option<String>,
    /// Accessibility label
    pub label: Option<String>,
    /// Whether icon is decorative only
    pub decorative: bool,
}

impl Icon {
    /// Create a new icon
    pub fn new(id: IconId) -> Self {
        Self {
            id,
            set: IconSet::default(),
            size: IconSize::default(),
            color: None,
            label: None,
            decorative: false,
        }
    }

    /// Set icon set
    pub fn with_set(mut self, set: IconSet) -> Self {
        self.set = set;
        self
    }

    /// Set size
    pub fn with_size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Set color
    pub fn with_color(mut self, color: &str) -> Self {
        self.color = Some(color.to_string());
        self
    }

    /// Set accessibility label
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Mark as decorative (no label needed)
    pub fn decorative(mut self) -> Self {
        self.decorative = true;
        self
    }

    /// Get SVG path data for the icon
    pub fn to_svg(&self) -> Option<&'static str> {
        get_icon_path(&self.id)
    }

    /// Get CSS classes for the icon
    pub fn to_css_classes(&self) -> String {
        format!(
            "icon icon-{} icon-{} {} {}",
            self.id.as_str(),
            self.size.as_str(),
            if self.set == IconSet::Lucide { "lucide" } else { "" },
            if self.decorative { "icon-decorative" } else { "" }
        )
    }
}

/// Icon identifiers (comprehensive set)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IconId {
    // Connection & Network
    Server,
    Database,
    Globe,
    Network,
    Wifi,
    WifiOff,
    Ethernet,
    Router,
    Cloud,
    CloudOffline,
    CloudUpload,
    CloudDownload,

    // Actions
    Connect,
    Disconnect,
    Refresh,
    Reload,
    Play,
    Pause,
    Stop,
    Restart,

    // Navigation
    Home,
    Back,
    Forward,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Menu,
    Close,
    Maximize,
    Minimize,
    Expand,
    Collapse,

    // Files & Folders
    Folder,
    FolderOpen,
    File,
    FileText,
    FileCode,
    FileJson,
    FileKey,
    FileLock,
    FilePlus,
    FileMinus,
    FileX,
    Files,
    Upload,
    Download,
    Import,
    Export,
    Save,
    Trash,

    // Terminal & Code
    Terminal,
    Code,
    Code2,
    Braces,
    Brackets,
    Command,
    TerminalSquare,
    Prompt,
    Cursor,

    // Settings & Tools
    Settings,
    Gear,
    Sliders,
    Filter,
    Search,
    ZoomIn,
    ZoomOut,
    Wrench,
    Tool,
    Hammer,
    Cog,

    // Security
    Lock,
    Unlock,
    Key,
    Shield,
    ShieldCheck,
    ShieldAlert,
    ShieldOff,
    Fingerprint,
    Eye,
    EyeOff,
    ScanFace,

    // Status & Feedback
    Check,
    CheckCircle,
    X,
    XCircle,
    AlertCircle,
    AlertTriangle,
    AlertOctagon,
    Info,
    HelpCircle,
    HelpOctagon,
    Bell,
    BellOff,
    BellRing,
    Badge,

    // User & Account
    User,
    Users,
    UserPlus,
    UserMinus,
    UserCheck,
    UserX,
    UserCircle,
    LogIn,
    LogOut,
    Account,
    Profile,

    // Communication
    Mail,
    MessageSquare,
    MessageCircle,
    Chat,
    Phone,
    Video,
    Share,
    Share2,
    Link,
    Unlink,
    ExternalLink,
    Copy,
    Clipboard,

    // Time
    Clock,
    Calendar,
    CalendarClock,
    CalendarCheck,
    CalendarX,
    CalendarPlus,
    Timer,
    Stopwatch,
    Hourglass,
    History,

    // Media
    Image,
    Video2,
    Music,
    Mic,
    Volume,
    Volume1,
    Volume2,
    VolumeX,
    VolumeOff,

    // Layout
    Layout,
    LayoutGrid,
    LayoutList,
    LayoutTemplate,
    Sidebar,
    SidebarLeft,
    SidebarRight,
    PanelLeft,
    PanelRight,
    Split,
    Columns,
    Rows,
    Grid,
    List,
    ListOrdered,

    // Indicators
    Circle,
    Square,
    Triangle,
    Star,
    Heart,
    ThumbsUp,
    ThumbsDown,
    Flag,
    Bookmark,
    Pin,
    Zap,
    Bolt,
    Activity,
    Pulse,

    // Editor
    Edit,
    Edit2,
    Edit3,
    Pen,
    Pencil,
    Highlighter,
    Eraser,
    Type,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,

    // Data & Analytics
    BarChart,
    BarChart2,
    LineChart,
    PieChart,
    TrendingUp,
    TrendingDown,
    Activity2,
    Gauge,
    Percent,
    DollarSign,
    CreditCard,
    Wallet,

    // SSH Specific
    Ssh,
    KeyPair,
    Certificate,
    Tunnel,
    Port,
    PortForward,
    Sftp,
    Scp,
    Shell,
    Bash,
    Zsh,
    Fish,

    // Other
    MoreHorizontal,
    MoreVertical,
    GripVertical,
    GripHorizontal,
    Dots,
    CircleDot,
    Layers,
    Stack,
    Box,
    Archive,
    Inbox,
    Send,
    Package,
    Map,
    MapPin,
    Navigation,
    Compass,
    Target,
    Crosshair,
    Aperture,
    Focus,
    Maximize2,
    Minimize2,
    Move,
    RotateCcw,
    RotateCw,
    FlipHorizontal,
    FlipVertical,

    // Accessibility
    A11y,
    ScreenReader,
    Keyboard,
    Mouse,
    MousePointer,
    MousePointer2,
    MousePointerClick,

    // Development
    GitBranch,
    GitCommit,
    GitMerge,
    GitPullRequest,
    GitFork,
    Github,
    Gitlab,
    Bitbucket,
    Container,
    Docker,
    Kubernetes,
    CloudNative,
    Api,
    Webhook,
    Plug,
    Puzzle,
    Binary,
    Cpu,
    Memory,
    HardDrive,
    Disc,
    Database2,
    Server2,
    Monitor,
    MonitorSpeaker,
    Tv,
    Smartphone,
    Tablet,
    Laptop,
    Desktop,
    Printer,
    Scanner,
    Headphones,
    Gamepad,
    Watch,
    Glasses,

    // Custom placeholder
    Custom(String),
}

impl IconId {
    /// Get string representation
    pub fn as_str(&self) -> String {
        match self {
            IconId::Custom(s) => s.clone(),
            _ => format!("{:?}", self).to_lowercase(),
        }
    }

    /// Get category
    pub fn category(&self) -> &'static str {
        use IconId::*;
        match self {
            Server | Database | Globe | Network | Wifi | WifiOff | Ethernet | Router
            | Cloud | CloudOffline | CloudUpload | CloudDownload => "network",

            Connect | Disconnect | Refresh | Reload | Play | Pause | Stop | Restart => "actions",

            Home | Back | Forward | ChevronLeft | ChevronRight | ChevronUp | ChevronDown
            | ArrowLeft | ArrowRight | ArrowUp | ArrowDown | Menu | Close | Maximize
            | Minimize | Expand | Collapse => "navigation",

            Folder | FolderOpen | File | FileText | FileCode | FileJson | FileKey | FileLock
            | FilePlus | FileMinus | FileX | Files | Upload | Download | Import | Export
            | Save | Trash => "files",

            Terminal | Code | Code2 | Braces | Brackets | Command | TerminalSquare | Prompt
            | Cursor => "terminal",

            Settings | Gear | Sliders | Filter | Search | ZoomIn | ZoomOut | Wrench | Tool
            | Hammer | Cog => "settings",

            Lock | Unlock | Key | Shield | ShieldCheck | ShieldAlert | ShieldOff | Fingerprint
            | Eye | EyeOff | ScanFace => "security",

            Check | CheckCircle | X | XCircle | AlertCircle | AlertTriangle | AlertOctagon
            | Info | HelpCircle | HelpOctagon | Bell | BellOff | BellRing | Badge => "status",

            User | Users | UserPlus | UserMinus | UserCheck | UserX | UserCircle | LogIn
            | LogOut | Account | Profile => "user",

            Mail | MessageSquare | MessageCircle | Chat | Phone | Video | Share | Share2
            | Link | Unlink | ExternalLink | Copy | Clipboard => "communication",

            Clock | Calendar | CalendarClock | CalendarCheck | CalendarX | CalendarPlus
            | Timer | Stopwatch | Hourglass | History => "time",

            Image | Video2 | Music | Mic | Volume | Volume1 | Volume2 | VolumeX | VolumeOff => "media",

            Layout | LayoutGrid | LayoutList | LayoutTemplate | Sidebar | SidebarLeft
            | SidebarRight | PanelLeft | PanelRight | Split | Columns | Rows | Grid | List
            | ListOrdered => "layout",

            Circle | Square | Triangle | Star | Heart | ThumbsUp | ThumbsDown | Flag
            | Bookmark | Pin | Zap | Bolt | Activity | Pulse => "indicators",

            Edit | Edit2 | Edit3 | Pen | Pencil | Highlighter | Eraser | Type | Bold | Italic
            | Underline | Strikethrough | AlignLeft | AlignCenter | AlignRight | AlignJustify => "editor",

            BarChart | BarChart2 | LineChart | PieChart | TrendingUp | TrendingDown
            | Activity2 | Gauge | Percent | DollarSign | CreditCard | Wallet => "data",

            Ssh | KeyPair | Certificate | Tunnel | Port | PortForward | Sftp | Scp | Shell
            | Bash | Zsh | Fish => "ssh",

            _ => "other",
        }
    }
}

/// Icon registry for managing icon sets
pub struct IconRegistry {
    sets: HashMap<IconSet, HashMap<IconId, &'static str>>,
    custom_icons: HashMap<String, String>,
}

impl IconRegistry {
    /// Create a new icon registry with default icons
    pub fn new() -> Self {
        let mut sets = HashMap::new();

        // Register Lucide icons (default)
        let mut lucide = HashMap::new();
        lucide.insert(IconId::Server, SERVER_ICON);
        lucide.insert(IconId::Terminal, TERMINAL_ICON);
        lucide.insert(IconId::Settings, SETTINGS_ICON);
        lucide.insert(IconId::Search, SEARCH_ICON);
        lucide.insert(IconId::Folder, FOLDER_ICON);
        lucide.insert(IconId::File, FILE_ICON);
        lucide.insert(IconId::Check, CHECK_ICON);
        lucide.insert(IconId::X, X_ICON);
        lucide.insert(IconId::Plus, PLUS_ICON);
        lucide.insert(IconId::Minus, MINUS_ICON);
        lucide.insert(IconId::ChevronRight, CHEVRON_RIGHT_ICON);
        lucide.insert(IconId::ChevronDown, CHEVRON_DOWN_ICON);
        lucide.insert(IconId::Lock, LOCK_ICON);
        lucide.insert(IconId::Key, KEY_ICON);
        lucide.insert(IconId::Cloud, CLOUD_ICON);
        lucide.insert(IconId::Home, HOME_ICON);
        lucide.insert(IconId::User, USER_ICON);
        lucide.insert(IconId::Bell, BELL_ICON);
        lucide.insert(IconId::Refresh, REFRESH_ICON);
        lucide.insert(IconId::Edit, EDIT_ICON);
        lucide.insert(IconId::Trash, TRASH_ICON);
        lucide.insert(IconId::Menu, MENU_ICON);
        lucide.insert(IconId::Close, CLOSE_ICON);
        lucide.insert(IconId::ArrowRight, ARROW_RIGHT_ICON);
        lucide.insert(IconId::Copy, COPY_ICON);
        lucide.insert(IconId::ExternalLink, EXTERNAL_LINK_ICON);
        lucide.insert(IconId::MoreVertical, MORE_VERTICAL_ICON);
        lucide.insert(IconId::LayoutGrid, LAYOUT_GRID_ICON);
        lucide.insert(IconId::List, LIST_ICON);
        lucide.insert(IconId::Maximize, MAXIMIZE_ICON);
        lucide.insert(IconId::Minimize, MINIMIZE_ICON);
        lucide.insert(IconId::Play, PLAY_ICON);
        lucide.insert(IconId::Pause, PAUSE_ICON);
        lucide.insert(IconId::Info, INFO_ICON);
        lucide.insert(IconId::AlertCircle, ALERT_CIRCLE_ICON);
        lucide.insert(IconId::AlertTriangle, ALERT_TRIANGLE_ICON);
        lucide.insert(IconId::Clock, CLOCK_ICON);
        lucide.insert(IconId::Calendar, CALENDAR_ICON);
        lucide.insert(IconId::Download, DOWNLOAD_ICON);
        lucide.insert(IconId::Upload, UPLOAD_ICON);
        lucide.insert(IconId::Shield, SHIELD_ICON);
        lucide.insert(IconId::Eye, EYE_ICON);
        lucide.insert(IconId::EyeOff, EYE_OFF_ICON);
        lucide.insert(IconId::Filter, FILTER_ICON);
        lucide.insert(IconId::Sort, SORT_ICON);
        lucide.insert(IconId::Star, STAR_ICON);
        lucide.insert(IconId::Heart, HEART_ICON);
        lucide.insert(IconId::Activity, ACTIVITY_ICON);
        lucide.insert(IconId::BarChart, BAR_CHART_ICON);
        lucide.insert(IconId::PieChart, PIE_CHART_ICON);
        lucide.insert(IconId::Wifi, WIFI_ICON);
        lucide.insert(IconId::WifiOff, WIFI_OFF_ICON);
        lucide.insert(IconId::Globe, GLOBE_ICON);
        lucide.insert(IconId::Database, DATABASE_ICON);
        lucide.insert(IconId::Cpu, CPU_ICON);
        lucide.insert(IconId::HardDrive, HARD_DRIVE_ICON);
        lucide.insert(IconId::Monitor, MONITOR_ICON);
        lucide.insert(IconId::Smartphone, SMARTPHONE_ICON);
        lucide.insert(IconId::Mail, MAIL_ICON);
        lucide.insert(IconId::MessageSquare, MESSAGE_SQUARE_ICON);
        lucide.insert(IconId::Phone, PHONE_ICON);
        lucide.insert(IconId::Video, VIDEO_ICON);
        lucide.insert(IconId::Share, SHARE_ICON);
        lucide.insert(IconId::Link, LINK_ICON);
        lucide.insert(IconId::Clipboard, CLIPBOARD_ICON);

        sets.insert(IconSet::Lucide, lucide);

        Self {
            sets,
            custom_icons: HashMap::new(),
        }
    }

    /// Register a custom icon
    pub fn register_custom(&mut self, id: &str, svg_path: &str) {
        self.custom_icons.insert(id.to_string(), svg_path.to_string());
    }

    /// Get SVG path for an icon
    pub fn get_path(&self, icon: &Icon) -> Option<&str> {
        match icon.id {
            IconId::Custom(ref id) => self.custom_icons.get(id).map(|s| s.as_str()),
            _ => self.sets
                .get(&icon.set)
                .and_then(|set| set.get(&icon.id).copied()),
        }
    }

    /// Check if icon exists
    pub fn has_icon(&self, icon: &Icon) -> bool {
        self.get_path(icon).is_some()
    }
}

impl Default for IconRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get icon path from global registry
fn get_icon_path(id: &IconId) -> Option<&'static str> {
    // Static registry lookup for common icons
    match id {
        IconId::Server => Some(SERVER_ICON),
        IconId::Terminal => Some(TERMINAL_ICON),
        IconId::Settings => Some(SETTINGS_ICON),
        IconId::Search => Some(SEARCH_ICON),
        IconId::Folder => Some(FOLDER_ICON),
        IconId::File => Some(FILE_ICON),
        IconId::Check => Some(CHECK_ICON),
        IconId::X => Some(X_ICON),
        IconId::Plus => Some(PLUS_ICON),
        IconId::Minus => Some(MINUS_ICON),
        IconId::ChevronRight => Some(CHEVRON_RIGHT_ICON),
        IconId::ChevronDown => Some(CHEVRON_DOWN_ICON),
        IconId::Lock => Some(LOCK_ICON),
        IconId::Key => Some(KEY_ICON),
        IconId::Cloud => Some(CLOUD_ICON),
        IconId::Home => Some(HOME_ICON),
        IconId::User => Some(USER_ICON),
        IconId::Bell => Some(BELL_ICON),
        IconId::Refresh => Some(REFRESH_ICON),
        IconId::Edit => Some(EDIT_ICON),
        IconId::Trash => Some(TRASH_ICON),
        IconId::Menu => Some(MENU_ICON),
        IconId::Close => Some(CLOSE_ICON),
        IconId::ArrowRight => Some(ARROW_RIGHT_ICON),
        IconId::Copy => Some(COPY_ICON),
        IconId::ExternalLink => Some(EXTERNAL_LINK_ICON),
        IconId::MoreVertical => Some(MORE_VERTICAL_ICON),
        IconId::LayoutGrid => Some(LAYOUT_GRID_ICON),
        IconId::List => Some(LIST_ICON),
        IconId::Maximize => Some(MAXIMIZE_ICON),
        IconId::Minimize => Some(MINIMIZE_ICON),
        IconId::Play => Some(PLAY_ICON),
        IconId::Pause => Some(PAUSE_ICON),
        IconId::Info => Some(INFO_ICON),
        IconId::AlertCircle => Some(ALERT_CIRCLE_ICON),
        IconId::AlertTriangle => Some(ALERT_TRIANGLE_ICON),
        IconId::Clock => Some(CLOCK_ICON),
        IconId::Calendar => Some(CALENDAR_ICON),
        IconId::Download => Some(DOWNLOAD_ICON),
        IconId::Upload => Some(UPLOAD_ICON),
        IconId::Shield => Some(SHIELD_ICON),
        IconId::Eye => Some(EYE_ICON),
        IconId::EyeOff => Some(EYE_OFF_ICON),
        IconId::Filter => Some(FILTER_ICON),
        IconId::Sort => Some(SORT_ICON),
        IconId::Star => Some(STAR_ICON),
        IconId::Heart => Some(HEART_ICON),
        IconId::Activity => Some(ACTIVITY_ICON),
        IconId::BarChart => Some(BAR_CHART_ICON),
        IconId::PieChart => Some(PIE_CHART_ICON),
        IconId::Wifi => Some(WIFI_ICON),
        IconId::WifiOff => Some(WIFI_OFF_ICON),
        IconId::Globe => Some(GLOBE_ICON),
        IconId::Database => Some(DATABASE_ICON),
        IconId::Cpu => Some(CPU_ICON),
        IconId::HardDrive => Some(HARD_DRIVE_ICON),
        IconId::Monitor => Some(MONITOR_ICON),
        IconId::Smartphone => Some(SMARTPHONE_ICON),
        IconId::Mail => Some(MAIL_ICON),
        IconId::MessageSquare => Some(MESSAGE_SQUARE_ICON),
        IconId::Phone => Some(PHONE_ICON),
        IconId::Video => Some(VIDEO_ICON),
        IconId::Share => Some(SHARE_ICON),
        IconId::Link => Some(LINK_ICON),
        IconId::Clipboard => Some(CLIPBOARD_ICON),
        _ => None,
    }
}

// SVG Path definitions for Lucide icons (simplified)
// These would be full SVG path data in production

const SERVER_ICON: &str = "M20 7H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2zM2 11h20";
const TERMINAL_ICON: &str = "m4 17 6-6-6-6m8 14h8";
const SETTINGS_ICON: &str = "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z";
const SEARCH_ICON: &str = "m21 21-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z";
const FOLDER_ICON: &str = "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z";
const FILE_ICON: &str = "M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z";
const CHECK_ICON: &str = "M20 6 9 17l-5-5";
const X_ICON: &str = "M18 6 6 18M6 6l12 12";
const PLUS_ICON: &str = "M12 5v14M5 12h14";
const MINUS_ICON: &str = "M5 12h14";
const CHEVRON_RIGHT_ICON: &str = "m9 18 6-6-6-6";
const CHEVRON_DOWN_ICON: &str = "m6 9 6 6 6-6";
const LOCK_ICON: &str = "M19 11H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-6a2 2 0 0 0-2-2zm0 0V7a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v4";
const KEY_ICON: &str = "m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4";
const CLOUD_ICON: &str = "M17.5 19c0-1.7-1.3-3-3-3h-11c-1.7 0-3 1.3-3 3 0 1.7 1.3 3 3 3h11c1.7 0 3-1.3 3-3z";
const HOME_ICON: &str = "m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z";
const USER_ICON: &str = "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2";
const BELL_ICON: &str = "M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9";
const REFRESH_ICON: &str = "M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8";
const EDIT_ICON: &str = "M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z";
const TRASH_ICON: &str = "M3 6h18m-2 0v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6";
const MENU_ICON: &str = "M4 12h16M4 18h16M4 6h16";
const CLOSE_ICON: &str = "M18 6 6 18M6 6l12 12";
const ARROW_RIGHT_ICON: &str = "M5 12h14M12 5l7 7-7 7";
const COPY_ICON: &str = "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2";
const EXTERNAL_LINK_ICON: &str = "M15 3h6v6M9 21 3 15l12-12";
const MORE_VERTICAL_ICON: &str = "M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0-5a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0 10a1 1 0 1 0 0-2 1 1 0 0 0 0 2z";
const LAYOUT_GRID_ICON: &str = "M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z";
const LIST_ICON: &str = "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01";
const MAXIMIZE_ICON: &str = "M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7";
const MINIMIZE_ICON: &str = "M4 14h6v6M20 10h-6V4";
const PLAY_ICON: &str = "m5 3 14 9-14 9V3z";
const PAUSE_ICON: &str = "M10 4H6v16h4V4zm8 0h-4v16h4V4z";
const INFO_ICON: &str = "M12 16v-4M12 8h.01M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z";
const ALERT_CIRCLE_ICON: &str = "M12 8v4m0 4h.01M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z";
const ALERT_TRIANGLE_ICON: &str = "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3z";
const CLOCK_ICON: &str = "M12 6v6l4 2m6-2A10 10 0 1 1 2 12a10 10 0 0 1 20 0z";
const CALENDAR_ICON: &str = "M8 2v4m8-4v4M3 10h18M4 10v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V10";
const DOWNLOAD_ICON: &str = "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4m4-5 5 5 5-5M12 15V3";
const UPLOAD_ICON: &str = "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4m14-7-5-5-5 5m5-5v12";
const SHIELD_ICON: &str = "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z";
const EYE_ICON: &str = "M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z";
const EYE_OFF_ICON: &str = "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24";
const FILTER_ICON: &str = "M22 3H2l8 9.46V19l4 2v-8.54L22 3z";
const SORT_ICON: &str = "M3 16h18M3 12h12M3 8h6";
const STAR_ICON: &str = "m12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z";
const HEART_ICON: &str = "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z";
const ACTIVITY_ICON: &str = "M22 12h-4l-3 9L9 3l-3 9H2";
const BAR_CHART_ICON: &str = "M18 20V10M12 20V4M6 20v-6";
const PIE_CHART_ICON: &str = "M21.21 15.89A10 10 0 1 1 8 2.83";
const WIFI_ICON: &str = "M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01";
const WIFI_OFF_ICON: &str = "M12 20h.01M8.53 16.11a6 6 0 0 1 6.95 0M6 13.55A11 11 0 0 1 12 11c2.64 0 5.08.96 7 2.54M3 3l18 18";
const GLOBE_ICON: &str = "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zM2 12h20";
const DATABASE_ICON: &str = "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83";
const CPU_ICON: &str = "M4 4h16v16H4z";
const HARD_DRIVE_ICON: &str = "M22 12h-4l-3 9L9 3l-3 9H2";
const MONITOR_ICON: &str = "M20 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2zM12 17v4";
const SMARTPHONE_ICON: &str = "M17 2H7a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM12 18h.01";
const MAIL_ICON: &str = "M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z";
const MESSAGE_SQUARE_ICON: &str = "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z";
const PHONE_ICON: &str = "M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z";
const VIDEO_ICON: &str = "m22 8-6 4 6 4V8zM2 5h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z";
const SHARE_ICON: &str = "M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8m-4-6-4-4-4 4m4-4v13";
const LINK_ICON: &str = "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71";
const CLIPBOARD_ICON: &str = "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_creation() {
        let icon = Icon::new(IconId::Server)
            .with_size(IconSize::Large)
            .with_color("#FF0000")
            .with_label("Server icon")
            .decorative();

        assert_eq!(icon.size, IconSize::Large);
        assert_eq!(icon.color, Some("#FF0000".to_string()));
        assert!(icon.decorative);
    }

    #[test]
    fn test_icon_size_px() {
        assert_eq!(IconSize::Xs.as_px(), 12);
        assert_eq!(IconSize::Medium.as_px(), 20);
        assert_eq!(IconSize::Xxl.as_px(), 48);
    }

    #[test]
    fn test_icon_set() {
        let icon = Icon::new(IconId::Server)
            .with_set(IconSet::Lucide);

        assert!(icon.to_svg().is_some());
    }

    #[test]
    fn test_icon_id_category() {
        assert_eq!(IconId::Server.category(), "network");
        assert_eq!(IconId::Terminal.category(), "terminal");
        assert_eq!(IconId::Settings.category(), "settings");
    }

    #[test]
    fn test_icon_registry() {
        let registry = IconRegistry::new();
        let icon = Icon::new(IconId::Server);

        assert!(registry.has_icon(&icon));
        assert!(registry.get_path(&icon).is_some());
    }

    #[test]
    fn test_custom_icon() {
        let mut registry = IconRegistry::new();
        registry.register_custom("my-icon", "M10 10");

        let icon = Icon {
            id: IconId::Custom("my-icon".to_string()),
            set: IconSet::Custom,
            size: IconSize::Medium,
            color: None,
            label: None,
            decorative: false,
        };

        assert!(registry.has_icon(&icon));
    }

    #[test]
    fn test_css_classes() {
        let icon = Icon::new(IconId::Server)
            .with_size(IconSize::Large)
            .decorative();

        let classes = icon.to_css_classes();
        assert!(classes.contains("icon"));
        assert!(classes.contains("icon-server"));
        assert!(classes.contains("icon-decorative"));
    }
}
