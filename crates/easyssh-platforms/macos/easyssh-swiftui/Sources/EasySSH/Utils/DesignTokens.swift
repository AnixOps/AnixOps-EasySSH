import SwiftUI

// MARK: - Theme Manager
/// Central theme manager for EasySSH macOS app
/// Handles automatic dark/light mode detection and provides dynamic color resolution
@MainActor
class ThemeManager: ObservableObject {
    static let shared = ThemeManager()

    @Published var colorScheme: ColorScheme = .light
    @Published var accentColor: Color = .blue

    var isDarkMode: Bool {
        colorScheme == .dark
    }

    func resolveColor(_ token: ColorToken) -> Color {
        token.color(for: colorScheme)
    }
}

// MARK: - Color Tokens
/// Comprehensive color system for EasySSH
/// Supports automatic dark/light theme adaptation
enum ColorToken {
    // MARK: Neutral Colors (Gray Scale)
    case neutral0
    case neutral50
    case neutral100
    case neutral200
    case neutral300
    case neutral400
    case neutral500
    case neutral600
    case neutral700
    case neutral800
    case neutral900
    case neutral950

    // MARK: Brand Colors (Primary)
    case brand50
    case brand100
    case brand200
    case brand300
    case brand400
    case brand500
    case brand600
    case brand700
    case brand800
    case brand900
    case brand950

    // MARK: Semantic Colors
    case success50
    case success100
    case success200
    case success300
    case success400
    case success500
    case success600
    case success700
    case success800
    case success900
    case success950

    case warning50
    case warning100
    case warning200
    case warning300
    case warning400
    case warning500
    case warning600
    case warning700
    case warning800
    case warning900
    case warning950

    case error50
    case error100
    case error200
    case error300
    case error400
    case error500
    case error600
    case error700
    case error800
    case error900
    case error950

    case info50
    case info100
    case info200
    case info300
    case info400
    case info500
    case info600
    case info700
    case info800
    case info900
    case info950

    // MARK: Terminal Colors
    case terminalBlack
    case terminalRed
    case terminalGreen
    case terminalYellow
    case terminalBlue
    case terminalMagenta
    case terminalCyan
    case terminalWhite
    case terminalBrightBlack
    case terminalBrightRed
    case terminalBrightGreen
    case terminalBrightYellow
    case terminalBrightBlue
    case terminalBrightMagenta
    case terminalBrightCyan
    case terminalBrightWhite
    case terminalBackground
    case terminalForeground
    case terminalCursor
    case terminalSelection

    // MARK: Status Colors
    case statusOnline
    case statusOffline
    case statusConnecting
    case statusError
    case statusWarning

    // MARK: Background Colors
    case bgPrimary
    case bgSecondary
    case bgTertiary
    case bgElevated
    case bgOverlay
    case bgSidebar
    case bgTerminal

    // MARK: Text Colors
    case textPrimary
    case textSecondary
    case textTertiary
    case textQuaternary
    case textInverted
    case textTerminal

    // MARK: Border Colors
    case borderDefault
    case borderStrong
    case borderFocus
    case borderError
    case borderSuccess

    // MARK: Icon Colors
    case iconPrimary
    case iconSecondary
    case iconTertiary
    case iconInverted

    // MARK: Control Colors
    case controlDefault
    case controlHover
    case controlPressed
    case controlDisabled
    case controlSelected

    // MARK: Color Resolution
    func color(for scheme: ColorScheme) -> Color {
        let isDark = scheme == .dark

        switch self {
        // Neutral Scale
        case .neutral0:
            return isDark ? Color(hex: "#000000") : Color(hex: "#FFFFFF")
        case .neutral50:
            return isDark ? Color(hex: "#0A0A0A") : Color(hex: "#FAFAFA")
        case .neutral100:
            return isDark ? Color(hex: "#1A1A1A") : Color(hex: "#F5F5F5")
        case .neutral200:
            return isDark ? Color(hex: "#2A2A2A") : Color(hex: "#E5E5E5")
        case .neutral300:
            return isDark ? Color(hex: "#404040") : Color(hex: "#D4D4D4")
        case .neutral400:
            return isDark ? Color(hex: "#525252") : Color(hex: "#A3A3A3")
        case .neutral500:
            return isDark ? Color(hex: "#737373") : Color(hex: "#737373")
        case .neutral600:
            return isDark ? Color(hex: "#A3A3A3") : Color(hex: "#525252")
        case .neutral700:
            return isDark ? Color(hex: "#D4D4D4") : Color(hex: "#404040")
        case .neutral800:
            return isDark ? Color(hex: "#E5E5E5") : Color(hex: "#2A2A2A")
        case .neutral900:
            return isDark ? Color(hex: "#F5F5F5") : Color(hex: "#1A1A1A")
        case .neutral950:
            return isDark ? Color(hex: "#FAFAFA") : Color(hex: "#0A0A0A")

        // Brand Colors (Blue)
        case .brand50:
            return Color(hex: isDark ? "#0F172A" : "#EFF6FF")
        case .brand100:
            return Color(hex: isDark ? "#1E3A5F" : "#DBEAFE")
        case .brand200:
            return Color(hex: isDark ? "#1E4A7C" : "#BFDBFE")
        case .brand300:
            return Color(hex: isDark ? "#2563A6" : "#93C5FD")
        case .brand400:
            return Color(hex: isDark ? "#3B82C6" : "#60A5FA")
        case .brand500:
            return Color(hex: "#3B82F6")
        case .brand600:
            return Color(hex: isDark ? "#60A5FA" : "#2563EB")
        case .brand700:
            return Color(hex: isDark ? "#93C5FD" : "#1D4ED8")
        case .brand800:
            return Color(hex: isDark ? "#BFDBFE" : "#1E40AF")
        case .brand900:
            return Color(hex: isDark ? "#DBEAFE" : "#1E3A8A")
        case .brand950:
            return Color(hex: isDark ? "#EFF6FF" : "#172554")

        // Success Colors (Green)
        case .success50:
            return Color(hex: isDark ? "#052E16" : "#F0FDF4")
        case .success100:
            return Color(hex: isDark ? "#14532D" : "#DCFCE7")
        case .success200:
            return Color(hex: isDark ? "#166534" : "#BBF7D0")
        case .success300:
            return Color(hex: isDark ? "#15803D" : "#86EFAC")
        case .success400:
            return Color(hex: isDark ? "#16A34A" : "#4ADE80")
        case .success500:
            return Color(hex: "#22C55E")
        case .success600:
            return Color(hex: isDark ? "#4ADE80" : "#16A34A")
        case .success700:
            return Color(hex: isDark ? "#86EFAC" : "#15803D")
        case .success800:
            return Color(hex: isDark ? "#BBF7D0" : "#166534")
        case .success900:
            return Color(hex: isDark ? "#DCFCE7" : "#14532D")
        case .success950:
            return Color(hex: isDark ? "#F0FDF4" : "#052E16")

        // Warning Colors (Amber)
        case .warning50:
            return Color(hex: isDark ? "#451A03" : "#FFFBEB")
        case .warning100:
            return Color(hex: isDark ? "#78350F" : "#FEF3C7")
        case .warning200:
            return Color(hex: isDark ? "#92400E" : "#FDE68A")
        case .warning300:
            return Color(hex: isDark ? "#B45309" : "#FCD34D")
        case .warning400:
            return Color(hex: isDark ? "#D97706" : "#FBBF24")
        case .warning500:
            return Color(hex: "#F59E0B")
        case .warning600:
            return Color(hex: isDark ? "#FBBF24" : "#D97706")
        case .warning700:
            return Color(hex: isDark ? "#FCD34D" : "#B45309")
        case .warning800:
            return Color(hex: isDark ? "#FDE68A" : "#92400E")
        case .warning900:
            return Color(hex: isDark ? "#FEF3C7" : "#78350F")
        case .warning950:
            return Color(hex: isDark ? "#FFFBEB" : "#451A03")

        // Error Colors (Red)
        case .error50:
            return Color(hex: isDark ? "#450A0A" : "#FEF2F2")
        case .error100:
            return Color(hex: isDark ? "#7F1D1D" : "#FEE2E2")
        case .error200:
            return Color(hex: isDark ? "#991B1B" : "#FECACA")
        case .error300:
            return Color(hex: isDark ? "#B91C1C" : "#FCA5A5")
        case .error400:
            return Color(hex: isDark ? "#DC2626" : "#F87171")
        case .error500:
            return Color(hex: "#EF4444")
        case .error600:
            return Color(hex: isDark ? "#F87171" : "#DC2626")
        case .error700:
            return Color(hex: isDark ? "#FCA5A5" : "#B91C1C")
        case .error800:
            return Color(hex: isDark ? "#FECACA" : "#991B1B")
        case .error900:
            return Color(hex: isDark ? "#FEE2E2" : "#7F1D1D")
        case .error950:
            return Color(hex: isDark ? "#FEF2F2" : "#450A0A")

        // Info Colors (Sky)
        case .info50:
            return Color(hex: isDark ? "#082F49" : "#F0F9FF")
        case .info100:
            return Color(hex: isDark ? "#0C4A6E" : "#E0F2FE")
        case .info200:
            return Color(hex: isDark ? "#075985" : "#BAE6FD")
        case .info300:
            return Color(hex: isDark ? "#0369A1" : "#7DD3FC")
        case .info400:
            return Color(hex: isDark ? "#0284C7" : "#38BDF8")
        case .info500:
            return Color(hex: "#0EA5E9")
        case .info600:
            return Color(hex: isDark ? "#38BDF8" : "#0284C7")
        case .info700:
            return Color(hex: isDark ? "#7DD3FC" : "#0369A1")
        case .info800:
            return Color(hex: isDark ? "#BAE6FD" : "#075985")
        case .info900:
            return Color(hex: isDark ? "#E0F2FE" : "#0C4A6E")
        case .info950:
            return Color(hex: isDark ? "#F0F9FF" : "#082F49")

        // Terminal Colors (xterm-256 compatible)
        case .terminalBlack:
            return Color(hex: "#000000")
        case .terminalRed:
            return Color(hex: "#CD3131")
        case .terminalGreen:
            return Color(hex: "#0DBC79")
        case .terminalYellow:
            return Color(hex: "#E5E510")
        case .terminalBlue:
            return Color(hex: "#2472C8")
        case .terminalMagenta:
            return Color(hex: "#BC3FBC")
        case .terminalCyan:
            return Color(hex: "#11A8CD")
        case .terminalWhite:
            return Color(hex: "#E5E5E5")
        case .terminalBrightBlack:
            return Color(hex: "#666666")
        case .terminalBrightRed:
            return Color(hex: "#F14C4C")
        case .terminalBrightGreen:
            return Color(hex: "#23D18B")
        case .terminalBrightYellow:
            return Color(hex: "#F5F543")
        case .terminalBrightBlue:
            return Color(hex: "#3B8EEA")
        case .terminalBrightMagenta:
            return Color(hex: "#D670D6")
        case .terminalBrightCyan:
            return Color(hex: "#29B8DB")
        case .terminalBrightWhite:
            return Color(hex: "#FFFFFF")
        case .terminalBackground:
            return isDark ? Color(hex: "#1E1E1E") : Color(hex: "#FFFFFF")
        case .terminalForeground:
            return isDark ? Color(hex: "#CCCCCC") : Color(hex: "#333333")
        case .terminalCursor:
            return Color(hex: "#AEAFAD")
        case .terminalSelection:
            return isDark ? Color(hex: "#264F78") : Color(hex: "#ADD6FF")

        // Status Colors
        case .statusOnline:
            return Color(hex: "#22C55E")
        case .statusOffline:
            return Color(hex: "#6B7280")
        case .statusConnecting:
            return Color(hex: "#F59E0B")
        case .statusError:
            return Color(hex: "#EF4444")
        case .statusWarning:
            return Color(hex: "#F59E0B")

        // Background Colors
        case .bgPrimary:
            return isDark ? Color(hex: "#0A0A0A") : Color(hex: "#FFFFFF")
        case .bgSecondary:
            return isDark ? Color(hex: "#141414") : Color(hex: "#F5F5F5")
        case .bgTertiary:
            return isDark ? Color(hex: "#1E1E1E") : Color(hex: "#E5E5E5")
        case .bgElevated:
            return isDark ? Color(hex: "#262626") : Color(hex: "#FFFFFF")
        case .bgOverlay:
            return isDark ? Color(hex: "#000000").opacity(0.7) : Color(hex: "#000000").opacity(0.4)
        case .bgSidebar:
            return isDark ? Color(hex: "#111111") : Color(hex: "#F0F0F0")
        case .bgTerminal:
            return isDark ? Color(hex: "#1E1E1E") : Color(hex: "#FFFFFF")

        // Text Colors
        case .textPrimary:
            return isDark ? Color(hex: "#FAFAFA") : Color(hex: "#0A0A0A")
        case .textSecondary:
            return isDark ? Color(hex: "#A3A3A3") : Color(hex: "#525252")
        case .textTertiary:
            return isDark ? Color(hex: "#737373") : Color(hex: "#737373")
        case .textQuaternary:
            return isDark ? Color(hex: "#525252") : Color(hex: "#A3A3A3")
        case .textInverted:
            return isDark ? Color(hex: "#0A0A0A") : Color(hex: "#FAFAFA")
        case .textTerminal:
            return isDark ? Color(hex: "#CCCCCC") : Color(hex: "#333333")

        // Border Colors
        case .borderDefault:
            return isDark ? Color(hex: "#2A2A2A") : Color(hex: "#E5E5E5")
        case .borderStrong:
            return isDark ? Color(hex: "#404040") : Color(hex: "#D4D4D4")
        case .borderFocus:
            return Color(hex: "#3B82F6")
        case .borderError:
            return Color(hex: "#EF4444")
        case .borderSuccess:
            return Color(hex: "#22C55E")

        // Icon Colors
        case .iconPrimary:
            return isDark ? Color(hex: "#E5E5E5") : Color(hex: "#2A2A2A")
        case .iconSecondary:
            return isDark ? Color(hex: "#A3A3A3") : Color(hex: "#525252")
        case .iconTertiary:
            return isDark ? Color(hex: "#737373") : Color(hex: "#737373")
        case .iconInverted:
            return isDark ? Color(hex: "#2A2A2A") : Color(hex: "#E5E5E5")

        // Control Colors
        case .controlDefault:
            return isDark ? Color(hex: "#2A2A2A") : Color(hex: "#FFFFFF")
        case .controlHover:
            return isDark ? Color(hex: "#404040") : Color(hex: "#F5F5F5")
        case .controlPressed:
            return isDark ? Color(hex: "#525252") : Color(hex: "#E5E5E5")
        case .controlDisabled:
            return isDark ? Color(hex: "#1A1A1A") : Color(hex: "#F5F5F5")
        case .controlSelected:
            return Color(hex: "#3B82F6")
        }
    }
}

// MARK: - Spacing Tokens
/// Standardized spacing system for consistent layout
enum SpacingToken: CGFloat {
    /// 0pt - No spacing
    case none = 0
    /// 2pt - Extra small spacing
    case xs = 2
    /// 4pt - Small spacing
    case sm = 4
    /// 8pt - Medium small spacing
    case md = 8
    /// 12pt - Medium spacing
    case lg = 12
    /// 16pt - Large spacing
    case xl = 16
    /// 20pt - Extra large spacing
    case xl2 = 20
    /// 24pt - 2x extra large spacing
    case xl3 = 24
    /// 32pt - 3x extra large spacing
    case xl4 = 32
    /// 40pt - 4x extra large spacing
    case xl5 = 40
    /// 48pt - 5x extra large spacing
    case xl6 = 48
    /// 64pt - 6x extra large spacing
    case xl7 = 64
    /// 80pt - 7x extra large spacing
    case xl8 = 80
    /// 96pt - 8x extra large spacing
    case xl9 = 96

    var value: CGFloat {
        rawValue
    }
}

// MARK: - Font Tokens
/// Typography system for EasySSH
enum FontToken {
    /// Large title for app header
    case largeTitle
    /// Title for window headers
    case title
    /// Title 2 for section headers
    case title2
    /// Title 3 for subsection headers
    case title3
    /// Headline for emphasized text
    case headline
    /// Body text
    case body
    /// Callout for captions and labels
    case callout
    /// Subheadline for secondary text
    case subheadline
    /// Footnote for small text
    case footnote
    /// Caption for very small text
    case caption
    /// Caption 2 for tiny text
    case caption2

    /// Terminal fonts for code/monospace display
    case terminalLarge
    case terminalMedium
    case terminalSmall
    case terminalTiny

    var font: Font {
        switch self {
        case .largeTitle:
            return .system(size: 26, weight: .bold, design: .default)
        case .title:
            return .system(size: 22, weight: .bold, design: .default)
        case .title2:
            return .system(size: 18, weight: .semibold, design: .default)
        case .title3:
            return .system(size: 16, weight: .semibold, design: .default)
        case .headline:
            return .system(size: 14, weight: .semibold, design: .default)
        case .body:
            return .system(size: 14, weight: .regular, design: .default)
        case .callout:
            return .system(size: 13, weight: .regular, design: .default)
        case .subheadline:
            return .system(size: 12, weight: .regular, design: .default)
        case .footnote:
            return .system(size: 11, weight: .regular, design: .default)
        case .caption:
            return .system(size: 10, weight: .regular, design: .default)
        case .caption2:
            return .system(size: 9, weight: .regular, design: .default)
        case .terminalLarge:
            return .system(size: 16, weight: .regular, design: .monospaced)
        case .terminalMedium:
            return .system(size: 14, weight: .regular, design: .monospaced)
        case .terminalSmall:
            return .system(size: 12, weight: .regular, design: .monospaced)
        case .terminalTiny:
            return .system(size: 10, weight: .regular, design: .monospaced)
        }
    }

    var nsFont: NSFont {
        switch self {
        case .largeTitle:
            return NSFont.systemFont(ofSize: 26, weight: .bold)
        case .title:
            return NSFont.systemFont(ofSize: 22, weight: .bold)
        case .title2:
            return NSFont.systemFont(ofSize: 18, weight: .semibold)
        case .title3:
            return NSFont.systemFont(ofSize: 16, weight: .semibold)
        case .headline:
            return NSFont.systemFont(ofSize: 14, weight: .semibold)
        case .body:
            return NSFont.systemFont(ofSize: 14, weight: .regular)
        case .callout:
            return NSFont.systemFont(ofSize: 13, weight: .regular)
        case .subheadline:
            return NSFont.systemFont(ofSize: 12, weight: .regular)
        case .footnote:
            return NSFont.systemFont(ofSize: 11, weight: .regular)
        case .caption:
            return NSFont.systemFont(ofSize: 10, weight: .regular)
        case .caption2:
            return NSFont.systemFont(ofSize: 9, weight: .regular)
        case .terminalLarge:
            return NSFont.monospacedSystemFont(ofSize: 16, weight: .regular)
        case .terminalMedium:
            return NSFont.monospacedSystemFont(ofSize: 14, weight: .regular)
        case .terminalSmall:
            return NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        case .terminalTiny:
            return NSFont.monospacedSystemFont(ofSize: 10, weight: .regular)
        }
    }
}

// MARK: - Corner Radius Tokens
enum CornerRadiusToken: CGFloat {
    /// 0pt - No radius (sharp corners)
    case none = 0
    /// 2pt - Extra small radius
    case xs = 2
    /// 4pt - Small radius
    case sm = 4
    /// 6pt - Small medium radius
    case smMd = 6
    /// 8pt - Medium radius
    case md = 8
    /// 10pt - Medium large radius
    case mdLg = 10
    /// 12pt - Large radius
    case lg = 12
    /// 16pt - Extra large radius
    case xl = 16
    /// 20pt - 2x extra large radius
    case xl2 = 20
    /// 24pt - 3x extra large radius
    case xl3 = 24
    /// 9999pt - Full radius (circle/pill)
    case full = 9999

    var value: CGFloat {
        rawValue
    }
}

// MARK: - Shadow Tokens
/// Shadow system for elevation
enum ShadowToken {
    /// No shadow
    case none
    /// Small shadow for subtle elevation
    case sm
    /// Medium shadow for cards
    case md
    /// Large shadow for elevated elements
    case lg
    /// Extra large shadow for modals/overlays
    case xl

    var swiftUIShadow: (color: Color, radius: CGFloat, x: CGFloat, y: CGFloat) {
        switch self {
        case .none:
            return (color: .clear, radius: 0, x: 0, y: 0)
        case .sm:
            return (color: Color.black.opacity(0.05), radius: 2, x: 0, y: 1)
        case .md:
            return (color: Color.black.opacity(0.08), radius: 4, x: 0, y: 2)
        case .lg:
            return (color: Color.black.opacity(0.12), radius: 8, x: 0, y: 4)
        case .xl:
            return (color: Color.black.opacity(0.16), radius: 16, x: 0, y: 8)
        }
    }
}

// MARK: - Animation Tokens
/// Standard animation durations and curves
enum AnimationToken {
    /// Instant - 0 seconds
    case instant
    /// Fast - 0.1 seconds
    case fast
    /// Normal - 0.2 seconds
    case normal
    /// Slow - 0.3 seconds
    case slow
    /// Slower - 0.5 seconds
    case slower
    /// Very slow - 0.8 seconds
    case verySlow

    var duration: Double {
        switch self {
        case .instant: return 0.0
        case .fast: return 0.1
        case .normal: return 0.2
        case .slow: return 0.3
        case .slower: return 0.5
        case .verySlow: return 0.8
        }
    }

    var animation: Animation {
        switch self {
        case .instant:
            return .easeInOut(duration: 0.001)
        case .fast:
            return .easeInOut(duration: 0.1)
        case .normal:
            return .easeInOut(duration: 0.2)
        case .slow:
            return .easeInOut(duration: 0.3)
        case .slower:
            return .easeInOut(duration: 0.5)
        case .verySlow:
            return .easeInOut(duration: 0.8)
        }
    }

    var springAnimation: Animation {
        switch self {
        case .instant:
            return .spring(response: 0.001, dampingFraction: 1.0)
        case .fast:
            return .spring(response: 0.1, dampingFraction: 0.8)
        case .normal:
            return .spring(response: 0.2, dampingFraction: 0.8)
        case .slow:
            return .spring(response: 0.3, dampingFraction: 0.8)
        case .slower:
            return .spring(response: 0.5, dampingFraction: 0.8)
        case .verySlow:
            return .spring(response: 0.8, dampingFraction: 0.8)
        }
    }
}

// MARK: - Layout Constants
/// Standard layout measurements
enum LayoutConstant {
    /// Minimum touch target size (44pt for accessibility)
    static let minTouchTarget: CGFloat = 44
    /// Standard button height
    static let buttonHeight: CGFloat = 32
    /// Large button height
    static let buttonHeightLarge: CGFloat = 40
    /// Small button height
    static let buttonHeightSmall: CGFloat = 24
    /// Standard icon size
    static let iconSize: CGFloat = 16
    /// Small icon size
    static let iconSizeSmall: CGFloat = 12
    /// Large icon size
    static let iconSizeLarge: CGFloat = 20
    /// Sidebar width
    static let sidebarWidth: CGFloat = 260
    /// Collapsed sidebar width
    static let sidebarWidthCollapsed: CGFloat = 60
    /// Toolbar height
    static let toolbarHeight: CGFloat = 52
    /// Status bar height
    static let statusBarHeight: CGFloat = 28
    /// Input field height
    static let inputHeight: CGFloat = 28
    /// List row height
    static let listRowHeight: CGFloat = 44
    /// Tab height
    static let tabHeight: CGFloat = 36
    /// Modal minimum width
    static let modalMinWidth: CGFloat = 400
    /// Modal maximum width
    static let modalMaxWidth: CGFloat = 600
    /// Terminal padding
    static let terminalPadding: CGFloat = 8
    /// Window minimum width
    static let windowMinWidth: CGFloat = 800
    /// Window minimum height
    static let windowMinHeight: CGFloat = 600
}

// MARK: - Z-Index Levels
/// Standard z-index hierarchy
enum ZIndexLevel: Double {
    /// Base layer - 0
    case base = 0
    /// Content layer - 10
    case content = 10
    /// Sticky headers/footers - 20
    case sticky = 20
    /// Dropdown menus - 30
    case dropdown = 30
    /// Floating elements - 40
    case floating = 40
    /// Overlays - 50
    case overlay = 50
    /// Modals - 60
    case modal = 60
    /// Popovers/tooltips - 70
    case popover = 70
    /// Notifications - 80
    case notification = 80
    /// Highest priority - 100
    case highest = 100

    var value: Double {
        rawValue
    }
}

// MARK: - Color Hex Extension
extension Color {
    /// Initialize Color from hex string
    /// Supports 3, 4, 6, and 8 character hex strings with optional # prefix
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 3: // RGB (12-bit)
            (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6: // RGB (24-bit)
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB (32-bit)
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (255, 0, 0, 0)
        }

        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }
}

// MARK: - View Modifiers
extension View {
    /// Apply design token color
    func tokenColor(_ token: ColorToken, for scheme: ColorScheme) -> some View {
        self.foregroundColor(token.color(for: scheme))
    }

    /// Apply design token background color
    func tokenBackground(_ token: ColorToken, for scheme: ColorScheme) -> some View {
        self.background(token.color(for: scheme))
    }

    /// Apply design token font
    func tokenFont(_ token: FontToken) -> some View {
        self.font(token.font)
    }

    /// Apply design token padding
    func tokenPadding(_ token: SpacingToken) -> some View {
        self.padding(token.value)
    }

    /// Apply design token corner radius
    func tokenCornerRadius(_ token: CornerRadiusToken) -> some View {
        self.cornerRadius(token.value)
    }

    /// Apply design token shadow
    func tokenShadow(_ token: ShadowToken) -> some View {
        let shadow = token.swiftUIShadow
        return self.shadow(
            color: shadow.color,
            radius: shadow.radius,
            x: shadow.x,
            y: shadow.y
        )
    }

    /// Apply standard card styling
    func cardStyle(colorScheme: ColorScheme) -> some View {
        self
            .tokenBackground(.bgElevated, for: colorScheme)
            .tokenCornerRadius(.lg)
            .tokenShadow(.md)
    }

    /// Apply standard input styling
    func inputStyle(colorScheme: ColorScheme) -> some View {
        self
            .padding(.horizontal, SpacingToken.md.value)
            .frame(height: LayoutConstant.inputHeight)
            .background(
                RoundedRectangle(cornerRadius: CornerRadiusToken.sm.value)
                    .fill(ColorToken.controlDefault.color(for: colorScheme))
            )
            .overlay(
                RoundedRectangle(cornerRadius: CornerRadiusToken.sm.value)
                    .stroke(ColorToken.borderDefault.color(for: colorScheme), lineWidth: 1)
            )
    }

    /// Apply primary button styling
    func primaryButtonStyle(colorScheme: ColorScheme) -> some View {
        self
            .tokenFont(.headline)
            .tokenPadding(.md)
            .frame(height: LayoutConstant.buttonHeight)
            .background(ColorToken.brand500.color(for: colorScheme))
            .tokenCornerRadius(.md)
            .foregroundColor(.white)
    }

    /// Apply secondary button styling
    func secondaryButtonStyle(colorScheme: ColorScheme) -> some View {
        self
            .tokenFont(.headline)
            .tokenPadding(.md)
            .frame(height: LayoutConstant.buttonHeight)
            .tokenBackground(.controlDefault, for: colorScheme)
            .overlay(
                RoundedRectangle(cornerRadius: CornerRadiusToken.md.value)
                    .stroke(ColorToken.borderDefault.color(for: colorScheme), lineWidth: 1)
            )
            .tokenCornerRadius(.md)
            .tokenColor(.textPrimary, for: colorScheme)
    }
}

// MARK: - Preview
#Preview("Design Tokens - Light") {
    DesignTokensPreview()
        .environmentObject(ThemeManager())
}

#Preview("Design Tokens - Dark") {
    DesignTokensPreview()
        .preferredColorScheme(.dark)
        .environmentObject(ThemeManager())
}

/// Comprehensive preview view for all design tokens
struct DesignTokensPreview: View {
    @EnvironmentObject var themeManager: ThemeManager
    @Environment(\.colorScheme) var colorScheme

    var body: some View {
        ScrollView {
            VStack(spacing: SpacingToken.xl4.value) {
                // Header
                VStack(spacing: SpacingToken.md.value) {
                    Text("EasySSH Design Tokens")
                        .tokenFont(.largeTitle)
                        .tokenColor(.textPrimary, for: colorScheme)

                    Text("macOS Design System")
                        .tokenFont(.subheadline)
                        .tokenColor(.textSecondary, for: colorScheme)

                    HStack(spacing: SpacingToken.md.value) {
                        Label(colorScheme == .dark ? "Dark Mode" : "Light Mode", systemImage: colorScheme == .dark ? "moon.fill" : "sun.max.fill")
                            .tokenFont(.callout)
                            .tokenColor(.brand500, for: colorScheme)
                    }
                }
                .tokenPadding(.xl2)

                // Color Section
                VStack(alignment: .leading, spacing: SpacingToken.xl.value) {
                    SectionHeader("Neutral Colors")
                    NeutralColorsGrid()

                    SectionHeader("Brand Colors")
                    BrandColorsGrid()

                    SectionHeader("Semantic Colors")
                    SemanticColorsGrid()

                    SectionHeader("Terminal Colors")
                    TerminalColorsGrid()

                    SectionHeader("Background Colors")
                    BackgroundColorsGrid()

                    SectionHeader("Text Colors")
                    TextColorsGrid()
                }
                .tokenPadding(.xl2)

                // Typography Section
                VStack(alignment: .leading, spacing: SpacingToken.xl.value) {
                    SectionHeader("Typography")
                    TypographyPreview()
                }
                .tokenPadding(.xl2)

                // Spacing Section
                VStack(alignment: .leading, spacing: SpacingToken.xl.value) {
                    SectionHeader("Spacing")
                    SpacingPreview()
                }
                .tokenPadding(.xl2)

                // Components Section
                VStack(alignment: .leading, spacing: SpacingToken.xl.value) {
                    SectionHeader("Components")
                    ComponentsPreview()
                }
                .tokenPadding(.xl2)

                // Status Indicators
                VStack(alignment: .leading, spacing: SpacingToken.xl.value) {
                    SectionHeader("Status Indicators")
                    StatusPreview()
                }
                .tokenPadding(.xl2)
            }
        }
        .frame(minWidth: 800, minHeight: 600)
        .tokenBackground(.bgPrimary, for: colorScheme)
    }
}

struct SectionHeader: View {
    let title: String
    @Environment(\.colorScheme) var colorScheme

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .tokenFont(.title2)
            .tokenColor(.textPrimary, for: colorScheme)
    }
}

struct NeutralColorsGrid: View {
    @Environment(\.colorScheme) var colorScheme

    let tokens: [(ColorToken, String)] = [
        (.neutral0, "0"), (.neutral50, "50"), (.neutral100, "100"),
        (.neutral200, "200"), (.neutral300, "300"), (.neutral400, "400"),
        (.neutral500, "500"), (.neutral600, "600"), (.neutral700, "700"),
        (.neutral800, "800"), (.neutral900, "900"), (.neutral950, "950"),
    ]

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 80))], spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                ColorSwatch(token: token, label: label)
            }
        }
    }
}

struct BrandColorsGrid: View {
    let tokens: [(ColorToken, String)] = [
        (.brand50, "50"), (.brand100, "100"), (.brand200, "200"),
        (.brand300, "300"), (.brand400, "400"), (.brand500, "500"),
        (.brand600, "600"), (.brand700, "700"), (.brand800, "800"),
        (.brand900, "900"), (.brand950, "950"),
    ]

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 80))], spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                ColorSwatch(token: token, label: label)
            }
        }
    }
}

struct SemanticColorsGrid: View {
    let tokens: [(ColorToken, String)] = [
        (.success500, "Success"), (.warning500, "Warning"),
        (.error500, "Error"), (.info500, "Info"),
    ]

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 120))], spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                ColorSwatch(token: token, label: label)
            }
        }
    }
}

struct TerminalColorsGrid: View {
    let tokens: [(ColorToken, String)] = [
        (.terminalBlack, "Black"), (.terminalRed, "Red"),
        (.terminalGreen, "Green"), (.terminalYellow, "Yellow"),
        (.terminalBlue, "Blue"), (.terminalMagenta, "Magenta"),
        (.terminalCyan, "Cyan"), (.terminalWhite, "White"),
    ]

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 100))], spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                ColorSwatch(token: token, label: label, showLabel: true)
            }
        }
    }
}

struct BackgroundColorsGrid: View {
    @Environment(\.colorScheme) var colorScheme

    let tokens: [(ColorToken, String)] = [
        (.bgPrimary, "Primary"), (.bgSecondary, "Secondary"),
        (.bgTertiary, "Tertiary"), (.bgElevated, "Elevated"),
        (.bgSidebar, "Sidebar"), (.bgTerminal, "Terminal"),
    ]

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 120))], spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                VStack {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(token.color(for: colorScheme))
                        .frame(height: 60)
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(ColorToken.borderDefault.color(for: colorScheme), lineWidth: 1)
                        )
                    Text(label)
                        .tokenFont(.caption)
                        .tokenColor(.textSecondary, for: colorScheme)
                }
            }
        }
    }
}

struct TextColorsGrid: View {
    @Environment(\.colorScheme) var colorScheme

    let tokens: [(ColorToken, String)] = [
        (.textPrimary, "Primary"), (.textSecondary, "Secondary"),
        (.textTertiary, "Tertiary"), (.textQuaternary, "Quaternary"),
        (.textInverted, "Inverted"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                Text(label)
                    .tokenFont(.body)
                    .tokenColor(token, for: colorScheme)
            }
        }
    }
}

struct ColorSwatch: View {
    let token: ColorToken
    let label: String
    var showLabel: Bool = false
    @Environment(\.colorScheme) var colorScheme

    var body: some View {
        VStack(spacing: SpacingToken.sm.value) {
            RoundedRectangle(cornerRadius: 6)
                .fill(token.color(for: colorScheme))
                .frame(height: 40)
                .overlay(
                    RoundedRectangle(cornerRadius: 6)
                        .stroke(ColorToken.borderDefault.color(for: colorScheme), lineWidth: 1)
                )
            Text(label)
                .tokenFont(.caption)
                .tokenColor(.textSecondary, for: colorScheme)
        }
    }
}

struct TypographyPreview: View {
    @Environment(\.colorScheme) var colorScheme

    let samples: [(FontToken, String)] = [
        (.largeTitle, "Large Title"),
        (.title, "Title"),
        (.title2, "Title 2"),
        (.title3, "Title 3"),
        (.headline, "Headline"),
        (.body, "Body text - The quick brown fox jumps over the lazy dog"),
        (.callout, "Callout - Additional information"),
        (.subheadline, "Subheadline - Secondary details"),
        (.footnote, "Footnote - Small print"),
        (.caption, "Caption - Very small"),
        (.terminalMedium, "Terminal - Monospace font for code"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: SpacingToken.lg.value) {
            ForEach(samples, id: \.1) { token, text in
                Text(text)
                    .tokenFont(token)
                    .tokenColor(.textPrimary, for: colorScheme)
            }
        }
    }
}

struct SpacingPreview: View {
    @Environment(\.colorScheme) var colorScheme

    let tokens: [(SpacingToken, String)] = [
        (.xs, "xs (2pt)"), (.sm, "sm (4pt)"), (.md, "md (8pt)"),
        (.lg, "lg (12pt)"), (.xl, "xl (16pt)"), (.xl2, "2xl (20pt)"),
        (.xl3, "3xl (24pt)"), (.xl4, "4xl (32pt)"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: SpacingToken.md.value) {
            ForEach(tokens, id: \.1) { token, label in
                HStack {
                    Rectangle()
                        .fill(ColorToken.brand500.color(for: colorScheme))
                        .frame(width: token.value, height: 20)
                    Text(label)
                        .tokenFont(.callout)
                        .tokenColor(.textSecondary, for: colorScheme)
                }
            }
        }
    }
}

struct ComponentsPreview: View {
    @Environment(\.colorScheme) var colorScheme

    var body: some View {
        VStack(alignment: .leading, spacing: SpacingToken.xl.value) {
            // Buttons
            HStack(spacing: SpacingToken.md.value) {
                Button("Primary") {}
                    .primaryButtonStyle(colorScheme: colorScheme)

                Button("Secondary") {}
                    .secondaryButtonStyle(colorScheme: colorScheme)
            }

            // Input
            Text("Input Field")
                .inputStyle(colorScheme: colorScheme)

            // Card
            VStack(alignment: .leading, spacing: SpacingToken.md.value) {
                Text("Card Title")
                    .tokenFont(.headline)
                Text("Card content with descriptive text")
                    .tokenFont(.body)
                    .tokenColor(.textSecondary, for: colorScheme)
            }
            .tokenPadding(.xl)
            .frame(maxWidth: .infinity, alignment: .leading)
            .cardStyle(colorScheme: colorScheme)

            // Status badges
            HStack(spacing: SpacingToken.md.value) {
                StatusBadge(text: "Online", color: .statusOnline)
                StatusBadge(text: "Offline", color: .statusOffline)
                StatusBadge(text: "Connecting", color: .statusConnecting)
                StatusBadge(text: "Error", color: .statusError)
            }
        }
    }
}

struct StatusBadge: View {
    let text: String
    let color: ColorToken
    @Environment(\.colorScheme) var colorScheme

    var body: some View {
        HStack(spacing: SpacingToken.sm.value) {
            Circle()
                .fill(color.color(for: colorScheme))
                .frame(width: 8, height: 8)
            Text(text)
                .tokenFont(.caption)
                .tokenColor(.textSecondary, for: colorScheme)
        }
        .tokenPadding(.sm)
        .background(
            Capsule()
                .fill(ColorToken.bgTertiary.color(for: colorScheme))
        )
    }
}

struct StatusPreview: View {
    @Environment(\.colorScheme) var colorScheme

    var body: some View {
        VStack(alignment: .leading, spacing: SpacingToken.lg.value) {
            StatusRow(icon: "checkmark.circle.fill", text: "Connected", color: .statusOnline)
            StatusRow(icon: "xmark.circle.fill", text: "Disconnected", color: .statusOffline)
            StatusRow(icon: "arrow.triangle.2.circlepath", text: "Connecting...", color: .statusConnecting)
            StatusRow(icon: "exclamationmark.triangle.fill", text: "Connection Error", color: .statusError)
        }
    }
}

struct StatusRow: View {
    let icon: String
    let text: String
    let color: ColorToken
    @Environment(\.colorScheme) var colorScheme

    var body: some View {
        HStack(spacing: SpacingToken.md.value) {
            Image(systemName: icon)
                .foregroundColor(color.color(for: colorScheme))
            Text(text)
                .tokenFont(.body)
                .tokenColor(.textPrimary, for: colorScheme)
        }
    }
}
