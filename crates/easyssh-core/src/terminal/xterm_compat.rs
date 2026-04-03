//! xterm.js 兼容性层
//! 提供完整的 xterm 转义序列解析和模拟

#[allow(unused_imports)]
use std::sync::Arc;

/// xterm 兼容模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XtermMode {
    /// VT100 基础模式
    VT100,
    /// VT220 扩展模式
    VT220,
    /// xterm 完全兼容模式
    Xterm256,
    /// xterm 真彩色模式
    XtermTrueColor,
}

impl Default for XtermMode {
    fn default() -> Self {
        XtermMode::Xterm256
    }
}

/// xterm 兼容性处理器
pub struct XtermCompat {
    mode: XtermMode,
    /// 当前解析状态
    state: ParseState,
    /// 解析缓冲区
    buffer: Vec<u8>,
    /// 当前光标位置
    cursor_row: u16,
    cursor_col: u16,
    /// 屏幕尺寸
    rows: u16,
    cols: u16,
    /// 滚动区域
    scroll_top: u16,
    scroll_bottom: u16,
    /// 当前文本属性
    attributes: TextAttributes,
    /// 保存的光标位置（用于 ESC 7/8）
    saved_cursor: Option<(u16, u16)>,
    /// 备用屏幕缓冲区标志
    in_alternate_screen: bool,
    /// 鼠标模式
    mouse_mode: MouseMode,
    /// 标题栈
    title_stack: Vec<String>,
    /// 特征标志
    features: XtermFeatures,
}

/// 解析状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    /// 正常状态
    Normal,
    /// 收到 ESC
    Escape,
    /// CSI 序列 (ESC [)
    CSI,
    /// OSC 序列 (ESC ])
    OSC,
    /// DCS 序列 (ESC P)
    DCS,
    /// APC 序列 (ESC _)
    APC,
    /// PM 序列 (ESC ^)
    PM,
    /// SOS 序列 (ESC X)
    SOS,
}

/// 文本属性
#[derive(Debug, Clone, Copy, Default)]
pub struct TextAttributes {
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub blink: bool,
    pub inverse: bool,
    pub invisible: bool,
}

/// 颜色定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// 默认颜色
    Default,
    /// 16色 ANSI
    Ansi(u8),
    /// 256色
    Indexed(u8),
    /// 真彩色 RGB
    RGB(u8, u8, u8),
}

impl Color {
    /// 转换为 u32 RGB
    pub fn to_u32(&self) -> u32 {
        match self {
            Color::Default => 0,
            Color::Ansi(n) => Self::ansi_to_rgb(*n),
            Color::Indexed(n) => Self::indexed_to_rgb(*n),
            Color::RGB(r, g, b) => ((*r as u32) << 16) | ((*g as u32) << 8) | (*b as u32),
        }
    }

    fn ansi_to_rgb(n: u8) -> u32 {
        // 标准 ANSI 16色到 RGB 映射
        let colors: [u32; 16] = [
            0x000000, // Black
            0xCD0000, // Red
            0x00CD00, // Green
            0xCDCD00, // Yellow
            0x0000EE, // Blue
            0xCD00CD, // Magenta
            0x00CDCD, // Cyan
            0xE5E5E5, // White
            0x7F7F7F, // Bright Black
            0xFF0000, // Bright Red
            0x00FF00, // Bright Green
            0xFFFF00, // Bright Yellow
            0x5C5CFF, // Bright Blue
            0xFF00FF, // Bright Magenta
            0x00FFFF, // Bright Cyan
            0xFFFFFF, // Bright White
        ];
        colors[n as usize % 16]
    }

    fn indexed_to_rgb(n: u8) -> u32 {
        if n < 16 {
            return Self::ansi_to_rgb(n);
        }
        if n < 232 {
            // 216色立方
            let r = ((n - 16) / 36) as u32;
            let g = (((n - 16) % 36) / 6) as u32;
            let b = ((n - 16) % 6) as u32;

            let r = if r == 0 { 0 } else { r * 40 + 55 };
            let g = if g == 0 { 0 } else { g * 40 + 55 };
            let b = if b == 0 { 0 } else { b * 40 + 55 };

            (r << 16) | (g << 8) | b
        } else {
            // 24级灰度
            let gray = 8 + (n - 232) as u32 * 10;
            (gray << 16) | (gray << 8) | gray
        }
    }
}

/// 鼠标模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// 无鼠标支持
    None,
    /// X10 模式
    X10,
    /// VT200 模式
    VT200,
    /// VT200 高亮模式
    VT200Highlight,
    /// 按钮事件追踪
    ButtonEvent,
    /// 任意事件追踪
    AnyEvent,
    /// SGR 扩展模式
    SGR,
    /// URXVT 扩展模式
    URXVT,
}

impl Default for MouseMode {
    fn default() -> Self {
        MouseMode::None
    }
}

/// xterm 特性标志
#[derive(Debug, Clone, Copy, Default)]
pub struct XtermFeatures {
    pub bracketed_paste: bool,
    pub focus_events: bool,
    pub mouse_reporting: bool,
    pub alt_screen: bool,
    pub cursor_blink: bool,
    pub cursor_visible: bool,
    pub auto_wrap: bool,
    pub reverse_wrap: bool,
    pub origin_mode: bool,
    pub insert_mode: bool,
    pub app_cursor_keys: bool,
    pub app_keypad: bool,
    pub scroll_on_output: bool,
    pub scroll_on_key: bool,
    pub lf_notspecial: bool,
}

/// 转义序列
#[derive(Debug, Clone)]
pub enum EscapeSequence {
    /// 文本输出
    Text(String),
    /// 光标上移
    CursorUp(u16),
    /// 光标下移
    CursorDown(u16),
    /// 光标前移
    CursorForward(u16),
    /// 光标后移
    CursorBackward(u16),
    /// 光标定位
    CursorPosition { row: u16, col: u16 },
    /// 光标保存
    SaveCursor,
    /// 光标恢复
    RestoreCursor,
    /// 擦除显示
    EraseDisplay(EraseMode),
    /// 擦除行
    EraseLine(EraseMode),
    /// 插入行
    InsertLines(u16),
    /// 删除行
    DeleteLines(u16),
    /// 滚动上
    ScrollUp(u16),
    /// 滚动下
    ScrollDown(u16),
    /// 设置图形渲染
    SetGraphicsRendition(Vec<GraphicRendition>),
    /// 设置模式
    SetMode(Vec<Mode>),
    /// 重置模式
    ResetMode(Vec<Mode>),
    /// 设置滚动区域
    SetScrollRegion { top: u16, bottom: u16 },
    /// 软重置
    SoftReset,
    /// 设置标题
    SetTitle {
        icon: Option<String>,
        window: Option<String>,
    },
    /// 设置图标标题
    SetIconTitle(String),
    /// 设置窗口标题
    SetWindowTitle(String),
    /// 设置调色板
    SetPalette { index: u8, color: Color },
    /// 响铃
    Bell,
    /// 退格
    Backspace,
    /// 水平制表
    HorizontalTab,
    /// 换行
    LineFeed,
    /// 回车
    CarriageReturn,
    /// 换页
    FormFeed,
    /// 垂直制表
    VerticalTab,
    /// 反向索引（RI）
    ReverseIndex,
    /// 删除字符
    DeleteChars(u16),
    /// 擦除字符
    EraseChars(u16),
    /// 未知序列
    Unknown(Vec<u8>),
}

/// 擦除模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    /// 光标到结尾
    ToEnd,
    /// 开头到光标
    FromStart,
    /// 整行/整屏
    All,
    /// 包含回滚缓冲区
    WithScrollback,
}

/// 图形渲染属性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicRendition {
    Reset,
    Bold,
    Faint,
    Italic,
    Underline,
    BlinkSlow,
    BlinkRapid,
    Inverse,
    Invisible,
    Strikethrough,
    PrimaryFont,
    Font(u8),
    Fraktur,
    DoubleUnderline,
    NormalIntensity,
    NotItalic,
    NotUnderline,
    NotBlink,
    ProportionalSpacing,
    NotInverse,
    Visible,
    NotStrikethrough,
    ForegroundColor(Color),
    BackgroundColor(Color),
    Framed,
    Encircled,
    Overlined,
    NotFramedOrEncircled,
    NotOverlined,
    UnderlineColor(Color),
    IdeogramUnderline,
    IdeogramDoubleUnderline,
    IdeogramOverline,
    IdeogramDoubleOverline,
    IdeogramStressMarking,
    NotIdeogram,
    Superscript,
    Subscript,
    NotSuperscriptOrSubscript,
}

/// 模式设置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 键盘锁定
    KeyboardAction,
    /// 插入模式
    Insert,
    /// 发送接收
    SendReceive,
    /// 控制字符显示
    Control8bit,
    /// 反向换行
    ReverseWrap,
    /// 相对原点模式
    Origin,
    /// 自动换行
    AutoWrap,
    /// 自动回车
    AutoCarriageReturn,
    /// 鼠标报告
    Mouse(u8),
    /// 备用屏幕缓冲区
    AlternateScreen,
    /// 应用光标键
    AppCursorKeys,
    /// 应用键盘
    AppKeypad,
    /// 括号粘贴模式
    BracketedPaste,
    /// 焦点事件
    FocusEvent,
    /// 鼠标高亮
    MouseHighlight,
    /// 按钮事件鼠标
    MouseButtonEvent,
    /// 任意事件鼠标
    MouseAnyEvent,
    /// SGR 鼠标模式
    MouseSGR,
    /// 滚动时清屏
    ClearOnScroll,
    /// 反向色彩
    ReverseColor,
    /// 启用 8 位控制
    C1Print,
}

impl XtermCompat {
    /// 创建新的 xterm 兼容处理器
    pub fn new(mode: XtermMode, rows: u16, cols: u16) -> Self {
        Self {
            mode,
            state: ParseState::Normal,
            buffer: Vec::with_capacity(256),
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
            scroll_top: 0,
            scroll_bottom: rows - 1,
            attributes: TextAttributes::default(),
            saved_cursor: None,
            in_alternate_screen: false,
            mouse_mode: MouseMode::None,
            title_stack: Vec::new(),
            features: XtermFeatures::default(),
        }
    }

    /// 检查 xterm 兼容层是否可用
    pub fn is_available(&self) -> bool {
        // 只要创建了实例，就认为可用
        true
    }

    /// 解析输入数据
    pub fn parse(&mut self, data: &[u8]) -> Vec<EscapeSequence> {
        let mut sequences = Vec::new();
        let mut text_buffer = String::new();

        for &byte in data {
            match self.state {
                ParseState::Normal => {
                    match byte {
                        0x1B => {
                            // ESC
                            if !text_buffer.is_empty() {
                                sequences.push(EscapeSequence::Text(text_buffer.clone()));
                                text_buffer.clear();
                            }
                            self.state = ParseState::Escape;
                        }
                        0x07 => sequences.push(EscapeSequence::Bell),
                        0x08 => sequences.push(EscapeSequence::Backspace),
                        0x09 => sequences.push(EscapeSequence::HorizontalTab),
                        0x0A => sequences.push(EscapeSequence::LineFeed),
                        0x0C => sequences.push(EscapeSequence::FormFeed),
                        0x0D => sequences.push(EscapeSequence::CarriageReturn),
                        0x0B => sequences.push(EscapeSequence::VerticalTab),
                        0x00 | 0x7F => {} // NUL, DEL 忽略
                        _ if byte < 0x20 => {
                            // 其他控制字符
                            if !text_buffer.is_empty() {
                                sequences.push(EscapeSequence::Text(text_buffer.clone()));
                                text_buffer.clear();
                            }
                        }
                        _ => text_buffer.push(byte as char),
                    }
                }
                ParseState::Escape => {
                    match byte {
                        b'[' => {
                            self.state = ParseState::CSI;
                            self.buffer.clear();
                        }
                        b']' => {
                            self.state = ParseState::OSC;
                            self.buffer.clear();
                        }
                        b'P' => {
                            self.state = ParseState::DCS;
                            self.buffer.clear();
                        }
                        b'_' => {
                            self.state = ParseState::APC;
                            self.buffer.clear();
                        }
                        b'^' => {
                            self.state = ParseState::PM;
                            self.buffer.clear();
                        }
                        b'X' => {
                            self.state = ParseState::SOS;
                            self.buffer.clear();
                        }
                        // 简单转义序列
                        b'A' => {
                            sequences.push(EscapeSequence::CursorUp(1));
                            self.state = ParseState::Normal;
                        }
                        b'B' => {
                            sequences.push(EscapeSequence::CursorDown(1));
                            self.state = ParseState::Normal;
                        }
                        b'C' => {
                            sequences.push(EscapeSequence::CursorForward(1));
                            self.state = ParseState::Normal;
                        }
                        b'D' => {
                            sequences.push(EscapeSequence::CursorBackward(1));
                            self.state = ParseState::Normal;
                        }
                        b'E' => {
                            sequences.push(EscapeSequence::CursorPosition {
                                row: self.cursor_row + 2,
                                col: 1,
                            });
                            self.state = ParseState::Normal;
                        }
                        b'F' => {
                            sequences.push(EscapeSequence::CursorPosition {
                                row: self.cursor_row.saturating_sub(1),
                                col: 1,
                            });
                            self.state = ParseState::Normal;
                        }
                        b'G' => {
                            sequences.push(EscapeSequence::CursorPosition {
                                row: self.cursor_row + 1,
                                col: 1,
                            });
                            self.state = ParseState::Normal;
                        }
                        b'H' => {
                            sequences.push(EscapeSequence::CursorPosition { row: 1, col: 1 });
                            self.state = ParseState::Normal;
                        }
                        b'7' => {
                            sequences.push(EscapeSequence::SaveCursor);
                            self.saved_cursor = Some((self.cursor_row, self.cursor_col));
                            self.state = ParseState::Normal;
                        }
                        b'8' => {
                            sequences.push(EscapeSequence::RestoreCursor);
                            if let Some((row, col)) = self.saved_cursor {
                                self.cursor_row = row;
                                self.cursor_col = col;
                            }
                            self.state = ParseState::Normal;
                        }
                        b'c' => {
                            sequences.push(EscapeSequence::SoftReset);
                            self.state = ParseState::Normal;
                        }
                        b'M' => {
                            sequences.push(EscapeSequence::ReverseIndex);
                            self.state = ParseState::Normal;
                        }
                        _ => {
                            // 未知序列
                            self.state = ParseState::Normal;
                        }
                    }
                }
                ParseState::CSI => {
                    match byte {
                        0x40..=0x7E => {
                            // 最终字节
                            self.buffer.push(byte);
                            let buffer_copy = self.buffer.clone();
                            if let Some(seq) = self.parse_csi(&buffer_copy) {
                                sequences.push(seq);
                            }
                            self.buffer.clear();
                            self.state = ParseState::Normal;
                        }
                        0x00..=0x1F => {
                            // CSI 中的控制字符，通常是参数的一部分
                            self.buffer.push(byte);
                        }
                        _ => {
                            self.buffer.push(byte);
                        }
                    }
                }
                ParseState::OSC => {
                    if byte == 0x07 || (byte == 0x1B && self.buffer.last() == Some(&b'\\')) {
                        // OSC 结束
                        if let Some(seq) = self.parse_osc(&self.buffer) {
                            sequences.push(seq);
                        }
                        self.buffer.clear();
                        self.state = ParseState::Normal;
                    } else {
                        self.buffer.push(byte);
                    }
                }
                ParseState::DCS | ParseState::APC | ParseState::PM | ParseState::SOS => {
                    // 这些序列以 ST (ESC \\ 或 BEL) 结束
                    if byte == 0x07 || (byte == 0x1B && self.buffer.last() == Some(&b'\\')) {
                        self.buffer.clear();
                        self.state = ParseState::Normal;
                    } else {
                        self.buffer.push(byte);
                    }
                }
            }
        }

        // 处理剩余文本
        if !text_buffer.is_empty() {
            sequences.push(EscapeSequence::Text(text_buffer));
        }

        sequences
    }

    /// 解析 CSI 序列
    fn parse_csi(&mut self, data: &[u8]) -> Option<EscapeSequence> {
        if data.is_empty() {
            return None;
        }

        let final_byte = *data.last().unwrap();
        let params = &data[..data.len() - 1];

        let param_values: Vec<u16> = if params.is_empty() {
            vec![0]
        } else {
            String::from_utf8_lossy(params)
                .split(';')
                .map(|s| s.parse().unwrap_or(0))
                .collect()
        };

        match final_byte {
            b'A' => Some(EscapeSequence::CursorUp(
                param_values.first().copied().unwrap_or(1),
            )),
            b'B' => Some(EscapeSequence::CursorDown(
                param_values.first().copied().unwrap_or(1),
            )),
            b'C' => Some(EscapeSequence::CursorForward(
                param_values.first().copied().unwrap_or(1),
            )),
            b'D' => Some(EscapeSequence::CursorBackward(
                param_values.first().copied().unwrap_or(1),
            )),
            b'E' => Some(EscapeSequence::CursorPosition {
                row: self.cursor_row + param_values.first().copied().unwrap_or(1) + 1,
                col: 1,
            }),
            b'F' => Some(EscapeSequence::CursorPosition {
                row: self
                    .cursor_row
                    .saturating_sub(param_values.first().copied().unwrap_or(1)),
                col: 1,
            }),
            b'G' => Some(EscapeSequence::CursorPosition {
                row: self.cursor_row + 1,
                col: param_values.first().copied().unwrap_or(1),
            }),
            b'H' | b'f' => {
                let row = param_values.first().copied().unwrap_or(1);
                let col = param_values.get(1).copied().unwrap_or(1);
                Some(EscapeSequence::CursorPosition { row, col })
            }
            b'J' => {
                let mode = match param_values.first().copied().unwrap_or(0) {
                    0 => EraseMode::ToEnd,
                    1 => EraseMode::FromStart,
                    2 => EraseMode::All,
                    3 => EraseMode::WithScrollback,
                    _ => EraseMode::All,
                };
                Some(EscapeSequence::EraseDisplay(mode))
            }
            b'K' => {
                let mode = match param_values.first().copied().unwrap_or(0) {
                    0 => EraseMode::ToEnd,
                    1 => EraseMode::FromStart,
                    2 => EraseMode::All,
                    _ => EraseMode::All,
                };
                Some(EscapeSequence::EraseLine(mode))
            }
            b'L' => Some(EscapeSequence::InsertLines(
                param_values.first().copied().unwrap_or(1),
            )),
            b'M' => Some(EscapeSequence::DeleteLines(
                param_values.first().copied().unwrap_or(1),
            )),
            b'P' => Some(EscapeSequence::DeleteChars(
                param_values.first().copied().unwrap_or(1),
            )),
            b'S' => Some(EscapeSequence::ScrollUp(
                param_values.first().copied().unwrap_or(1),
            )),
            b'T' => Some(EscapeSequence::ScrollDown(
                param_values.first().copied().unwrap_or(1),
            )),
            b'X' => Some(EscapeSequence::EraseChars(
                param_values.first().copied().unwrap_or(1),
            )),
            b'd' => {
                let row = param_values.first().copied().unwrap_or(1);
                Some(EscapeSequence::CursorPosition {
                    row,
                    col: self.cursor_col + 1,
                })
            }
            b'h' => {
                let modes = param_values
                    .iter()
                    .filter_map(|&n| self.decode_mode(n, true))
                    .collect();
                Some(EscapeSequence::SetMode(modes))
            }
            b'l' => {
                let modes = param_values
                    .iter()
                    .filter_map(|&n| self.decode_mode(n, false))
                    .collect();
                Some(EscapeSequence::ResetMode(modes))
            }
            b'm' => {
                let attrs = self.parse_sgr(&param_values);
                Some(EscapeSequence::SetGraphicsRendition(attrs))
            }
            b'r' => {
                let top = param_values.first().copied().unwrap_or(1);
                let bottom = param_values.get(1).copied().unwrap_or(self.rows);
                Some(EscapeSequence::SetScrollRegion { top, bottom })
            }
            b's' => Some(EscapeSequence::SaveCursor),
            b'u' => Some(EscapeSequence::RestoreCursor),
            _ => Some(EscapeSequence::Unknown(data.to_vec())),
        }
    }

    /// 解析 OSC 序列
    fn parse_osc(&self, data: &[u8]) -> Option<EscapeSequence> {
        if data.is_empty() {
            return None;
        }

        let ps = data[0] - b'0';
        let text = String::from_utf8_lossy(&data[1..]);

        match ps {
            0 => {
                let parts: Vec<&str> = text.split(';').collect();
                Some(EscapeSequence::SetTitle {
                    icon: parts.first().map(|s| s.to_string()),
                    window: parts.get(1).map(|s| s.to_string()),
                })
            }
            1 => Some(EscapeSequence::SetIconTitle(text.to_string())),
            2 => Some(EscapeSequence::SetWindowTitle(text.to_string())),
            4 => {
                // 设置调色板
                if let Some((index, color)) = self.parse_palette(&text) {
                    Some(EscapeSequence::SetPalette { index, color })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 解析 SGR 参数
    fn parse_sgr(&self, params: &[u16]) -> Vec<GraphicRendition> {
        let mut attrs = Vec::new();
        let mut i = 0;

        while i < params.len() {
            let code = params[i];
            match code {
                0 => attrs.push(GraphicRendition::Reset),
                1 => attrs.push(GraphicRendition::Bold),
                2 => attrs.push(GraphicRendition::Faint),
                3 => attrs.push(GraphicRendition::Italic),
                4 => attrs.push(GraphicRendition::Underline),
                5 => attrs.push(GraphicRendition::BlinkSlow),
                6 => attrs.push(GraphicRendition::BlinkRapid),
                7 => attrs.push(GraphicRendition::Inverse),
                8 => attrs.push(GraphicRendition::Invisible),
                9 => attrs.push(GraphicRendition::Strikethrough),
                10..=19 => attrs.push(GraphicRendition::Font((code - 10) as u8)),
                20 => attrs.push(GraphicRendition::Fraktur),
                21 => attrs.push(GraphicRendition::DoubleUnderline),
                22 => attrs.push(GraphicRendition::NormalIntensity),
                23 => attrs.push(GraphicRendition::NotItalic),
                24 => attrs.push(GraphicRendition::NotUnderline),
                25 => attrs.push(GraphicRendition::NotBlink),
                26 => attrs.push(GraphicRendition::ProportionalSpacing),
                27 => attrs.push(GraphicRendition::NotInverse),
                28 => attrs.push(GraphicRendition::Visible),
                29 => attrs.push(GraphicRendition::NotStrikethrough),
                30..=37 => attrs.push(GraphicRendition::ForegroundColor(Color::Ansi(
                    (code - 30) as u8,
                ))),
                38 => {
                    // 扩展前景色
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                attrs.push(GraphicRendition::ForegroundColor(Color::Indexed(
                                    params[i + 2] as u8,
                                )));
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                attrs.push(GraphicRendition::ForegroundColor(Color::RGB(
                                    params[i + 2] as u8,
                                    params[i + 3] as u8,
                                    params[i + 4] as u8,
                                )));
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                39 => attrs.push(GraphicRendition::ForegroundColor(Color::Default)),
                40..=47 => attrs.push(GraphicRendition::BackgroundColor(Color::Ansi(
                    (code - 40) as u8,
                ))),
                48 => {
                    // 扩展背景色
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                attrs.push(GraphicRendition::BackgroundColor(Color::Indexed(
                                    params[i + 2] as u8,
                                )));
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                attrs.push(GraphicRendition::BackgroundColor(Color::RGB(
                                    params[i + 2] as u8,
                                    params[i + 3] as u8,
                                    params[i + 4] as u8,
                                )));
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                49 => attrs.push(GraphicRendition::BackgroundColor(Color::Default)),
                90..=97 => attrs.push(GraphicRendition::ForegroundColor(Color::Ansi(
                    (code - 82) as u8,
                ))),
                100..=107 => attrs.push(GraphicRendition::BackgroundColor(Color::Ansi(
                    (code - 92) as u8,
                ))),
                _ => {}
            }
            i += 1;
        }

        attrs
    }

    /// 解码模式
    fn decode_mode(&self, n: u16, set: bool) -> Option<Mode> {
        match n {
            2 => Some(if set {
                Mode::KeyboardAction
            } else {
                Mode::KeyboardAction
            }),
            4 => Some(if set { Mode::Insert } else { Mode::Insert }),
            6 => Some(if set { Mode::Origin } else { Mode::Origin }),
            7 => Some(if set { Mode::AutoWrap } else { Mode::AutoWrap }),
            12 => Some(if set {
                Mode::AppCursorKeys
            } else {
                Mode::AppCursorKeys
            }),
            20 => Some(if set {
                Mode::AutoCarriageReturn
            } else {
                Mode::AutoCarriageReturn
            }),
            25 => Some(if set {
                Mode::ReverseColor
            } else {
                Mode::ReverseColor
            }),
            47 | 1047 => Some(if set {
                Mode::AlternateScreen
            } else {
                Mode::AlternateScreen
            }),
            1000 => Some(Mode::Mouse(0)),
            1001 => Some(Mode::MouseHighlight),
            1002 => Some(Mode::MouseButtonEvent),
            1003 => Some(Mode::MouseAnyEvent),
            1004 => Some(if set {
                Mode::FocusEvent
            } else {
                Mode::FocusEvent
            }),
            1005 => Some(Mode::Mouse(5)),
            1006 => Some(Mode::MouseSGR),
            1015 => Some(Mode::Mouse(15)),
            1048 => Some(if set {
                Mode::AlternateScreen
            } else {
                Mode::AlternateScreen
            }),
            2004 => Some(if set {
                Mode::BracketedPaste
            } else {
                Mode::BracketedPaste
            }),
            _ => None,
        }
    }

    /// 解析调色板设置
    fn parse_palette(&self, text: &str) -> Option<(u8, Color)> {
        let parts: Vec<&str> = text.split(';').collect();
        if parts.len() >= 2 {
            let index = parts[0].parse().ok()?;
            let color = self.parse_color_spec(parts[1])?;
            Some((index, color))
        } else {
            None
        }
    }

    /// 解析颜色规范
    fn parse_color_spec(&self, spec: &str) -> Option<Color> {
        if let Some(rgb) = spec.strip_prefix("rgb:") {
            let parts: Vec<&str> = rgb.split('/').collect();
            if parts.len() == 3 {
                let r = u8::from_str_radix(parts[0], 16).ok()?;
                let g = u8::from_str_radix(parts[1], 16).ok()?;
                let b = u8::from_str_radix(parts[2], 16).ok()?;
                return Some(Color::RGB(r, g, b));
            }
        } else if let Some(hex) = spec.strip_prefix("#") {
            if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some(Color::RGB(r, g, b));
            }
        }
        None
    }

    /// 设置屏幕尺寸
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
        self.scroll_bottom = rows - 1;

        // 确保光标在有效范围内
        self.cursor_row = self.cursor_row.min(rows - 1);
        self.cursor_col = self.cursor_col.min(cols - 1);
    }

    /// 获取当前光标位置
    pub fn cursor_position(&self) -> (u16, u16) {
        (self.cursor_row, self.cursor_col)
    }

    /// 获取当前文本属性
    pub fn current_attributes(&self) -> TextAttributes {
        self.attributes
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.state = ParseState::Normal;
        self.buffer.clear();
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.attributes = TextAttributes::default();
        self.saved_cursor = None;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
        self.features = XtermFeatures::default();
    }

    /// 生成响应序列
    pub fn generate_response(&self, query: &[u8]) -> Option<Vec<u8>> {
        if query.starts_with(b"\x1b[5n") {
            // DSR - 设备状态报告
            return Some(b"\x1b[0n".to_vec()); // 设备OK
        }

        if query.starts_with(b"\x1b[6n") {
            // CPR - 光标位置报告
            let response = format!("\x1b[{};{}R", self.cursor_row + 1, self.cursor_col + 1);
            return Some(response.into_bytes());
        }

        None
    }
}

impl Default for XtermCompat {
    fn default() -> Self {
        Self::new(XtermMode::default(), 24, 80)
    }
}

/// 兼容模式检测器
pub struct XtermModeDetector;

impl XtermModeDetector {
    /// 检测终端类型
    pub fn detect(terminal_name: &str) -> XtermMode {
        let name = terminal_name.to_lowercase();

        if name.contains("256color") || name.contains("256-color") {
            XtermMode::Xterm256
        } else if name.contains("truecolor") || name.contains("24bit") || name.contains("24-bit") {
            XtermMode::XtermTrueColor
        } else if name.contains("xterm") || name.contains("rxvt") || name.contains("vte") {
            XtermMode::Xterm256
        } else if name.contains("vt220") || name.contains("vt200") {
            XtermMode::VT220
        } else if name.contains("vt100") {
            XtermMode::VT100
        } else {
            XtermMode::Xterm256 // 默认256色
        }
    }
}

/// 扩展 EscapeSequence 枚举
#[derive(Debug, Clone)]
pub enum EscapeSequenceExt {
    ReverseIndex,
    DeleteChars(u16),
    EraseChars(u16),
    SetWindowSize {
        rows: u16,
        cols: u16,
    },
    DeviceStatusReport,
    DeviceAttributes,
    SecondaryDeviceAttributes,
    TertiaryDeviceAttributes,
    RequestTerminalParameters,
    RequestTerminalSizeInPixels,
    RequestTerminalSizeInChars,
    RequestTextAreaSizeInPixels,
    RequestScreenSizeInPixels,
    RequestCharacterSize,
    RequestTextAreaSizeInChars,
    RequestStatusString(String),
    RequestChecksumOfRect {
        top: u16,
        left: u16,
        bottom: u16,
        right: u16,
        page: u8,
    },
    QuerySGR,
    QueryTabStops,
    QueryMode(Mode),
    QueryDECMode(Mode),
    QueryColor {
        index: u8,
        query_type: ColorQueryType,
    },
    QueryKeyboardType,
    QueryKeyboardModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorQueryType {
    Color,
    RGB,
}
