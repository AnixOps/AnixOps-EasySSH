#![allow(dead_code)]

//! High-Performance WebGL Terminal Renderer
//!
//! Features:
//! - 60fps (16ms frame time)
//! - WebGL-accelerated rendering via wry WebView
//! - xterm.js integration
//! - 256 colors + True Color support
//! - Powerline fonts + Nerd Fonts
//! - Smooth cursor blink (CSS animation)
//! - Optimized selection highlight
//! - Big data streaming (>10MB)

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::debug;
use wry::{WebView, WebViewBuilder};

/// Terminal color support level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSupport {
    Basic,      // 16 colors
    Extended,   // 256 colors
    TrueColor,  // 24-bit RGB
}

impl Default for ColorSupport {
    fn default() -> Self {
        ColorSupport::TrueColor
    }
}

/// Font configuration for terminal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub powerline: bool,
    pub nerd_fonts: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono, Cascadia Code, Consolas, monospace".to_string(),
            size: 14.0,
            line_height: 1.2,
            letter_spacing: 0.0,
            powerline: true,
            nerd_fonts: true,
        }
    }
}

/// Cursor style
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Line,
    Bar,
}

impl Default for CursorStyle {
    fn default() -> Self {
        CursorStyle::Block
    }
}

/// Cursor configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorConfig {
    pub style: CursorStyle,
    pub blink: bool,
    pub blink_interval_ms: u64,
    pub color: String,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: CursorStyle::Block,
            blink: true,
            blink_interval_ms: 530, // Default terminal blink rate
            color: "#aeafad".to_string(),
        }
    }
}

fn instant_now() -> Instant {
    Instant::now()
}

/// Terminal renderer performance stats
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub gpu_time_ms: f32,
    pub render_calls: u64,
    pub dropped_frames: u64,
    #[serde(skip, default = "instant_now")]
    pub last_update: Instant,
}

impl Default for RenderStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            gpu_time_ms: 0.0,
            render_calls: 0,
            dropped_frames: 0,
            last_update: Instant::now(),
        }
    }
}

/// High-performance terminal configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub color_support: ColorSupport,
    pub font: FontConfig,
    pub cursor: CursorConfig,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_lines: usize,
    pub allow_transparency: bool,
    pub gpu_acceleration: bool,
    pub target_fps: u32,
    pub vsync: bool,
    pub webgl2: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            color_support: ColorSupport::TrueColor,
            font: FontConfig::default(),
            cursor: CursorConfig::default(),
            cols: 80,
            rows: 24,
            scrollback_lines: 100_000,
            allow_transparency: false,
            gpu_acceleration: true,
            target_fps: 60,
            vsync: true,
            webgl2: true,
        }
    }
}

/// Terminal cell data for rendering
#[derive(Clone, Debug)]
pub struct TerminalCell {
    pub char: char,
    pub fg_color: [u8; 4],  // RGBA
    pub bg_color: [u8; 4],  // RGBA
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg_color: [200, 210, 220, 255],
            bg_color: [22, 25, 30, 255],
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            inverse: false,
        }
    }
}

/// Selection range in terminal
#[derive(Clone, Debug, Default)]
pub struct SelectionRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub active: bool,
}

/// Terminal screen buffer
pub struct TerminalBuffer {
    width: usize,
    height: usize,
    cells: Vec<TerminalCell>,
    scroll_offset: usize,
    selection: SelectionRange,
    dirty: bool,
}

impl TerminalBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![TerminalCell::default(); width * height];
        Self {
            width,
            height,
            cells,
            scroll_offset: 0,
            selection: SelectionRange::default(),
            dirty: true,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        let mut new_cells = vec![TerminalCell::default(); width * height];

        // Copy existing content
        let copy_height = self.height.min(height);
        let copy_width = self.width.min(width);

        for row in 0..copy_height {
            for col in 0..copy_width {
                let old_idx = row * self.width + col;
                let new_idx = row * width + col;
                if old_idx < self.cells.len() && new_idx < new_cells.len() {
                    new_cells[new_idx] = self.cells[old_idx].clone();
                }
            }
        }

        self.width = width;
        self.height = height;
        self.cells = new_cells;
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = TerminalCell::default();
        }
        self.dirty = true;
    }

    pub fn set_cell(&mut self, row: usize, col: usize, cell: TerminalCell) {
        if row < self.height && col < self.width {
            let idx = row * self.width + col;
            self.cells[idx] = cell;
            self.dirty = true;
        }
    }

    pub fn get_cell(&self, row: usize, col: usize) -> Option<&TerminalCell> {
        if row < self.height && col < self.width {
            let idx = row * self.width + col;
            self.cells.get(idx)
        } else {
            None
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let scroll = lines.min(self.height);
        let cells_to_remove = scroll * self.width;

        self.cells.drain(0..cells_to_remove);
        self.cells.resize(self.width * self.height, TerminalCell::default());
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

/// High-performance WebGL terminal renderer
pub struct WebGlTerminal {
    config: TerminalConfig,
    buffer: Arc<Mutex<TerminalBuffer>>,
    stats: Arc<Mutex<RenderStats>>,
    last_render: Instant,
    frame_time_target: Duration,
    webview_html: String,
}

impl WebGlTerminal {
    pub fn new(config: TerminalConfig) -> Self {
        let buffer = Arc::new(Mutex::new(TerminalBuffer::new(
            config.cols,
            config.rows,
        )));

        let stats = Arc::new(Mutex::new(RenderStats {
            last_update: Instant::now(),
            ..Default::default()
        }));

        let frame_time_target = Duration::from_millis(1000 / config.target_fps as u64);

        let webview_html = Self::generate_xterm_html(&config);

        Self {
            config,
            buffer,
            stats,
            last_render: Instant::now(),
            frame_time_target,
            webview_html,
        }
    }

    /// Generate optimized xterm.js HTML with WebGL addon
    fn generate_xterm_html(config: &TerminalConfig) -> String {
        let font_family = &config.font.family;
        let font_size = config.font.size;
        let line_height = config.font.line_height;
        let cursor_blink = config.cursor.blink;
        let cursor_style = match config.cursor.style {
            CursorStyle::Block => "block",
            CursorStyle::Line => "line",
            CursorStyle::Bar => "bar",
        };
        let scrollback = config.scrollback_lines;

        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/xterm@5.3.0/css/xterm.css">
    <script src="https://cdn.jsdelivr.net/npm/xterm@5.3.0/lib/xterm.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-webgl@0.16.0/lib/xterm-addon-webgl.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-web-links@0.9.0/lib/xterm-addon-web-links.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-search@0.13.0/lib/xterm-addon-search.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-unicode11@0.6.0/lib/xterm-addon-unicode11.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-fit@0.8.0/lib/xterm-addon-fit.min.js"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        html, body {{
            width: 100%;
            height: 100%;
            background: #16181d;
            overflow: hidden;
        }}
        #terminal {{
            width: 100%;
            height: 100%;
            padding: 4px;
        }}
        /* Optimized cursor blink animation for 60fps */
        @keyframes cursor-blink {{
            0%, 50% {{ opacity: 1; }}
            51%, 100% {{ opacity: 0; }}
        }}
        .xterm-cursor {{
            animation: cursor-blink {}ms infinite;
            will-change: opacity;
            transform: translateZ(0);
        }}
        /* GPU acceleration for selection */
        .xterm-selection {{
            will-change: transform;
            transform: translateZ(0);
        }}
        /* Hardware acceleration for rendering */
        .xterm-rows {{
            will-change: transform;
            transform: translateZ(0);
        }}
        /* Optimize font rendering */
        .xterm {{
            font-family: {};
            font-size: {}px;
            line-height: {};
            -webkit-font-smoothing: antialiased;
            -moz-osx-font-smoothing: grayscale;
            text-rendering: optimizeLegibility;
            font-feature-settings: "liga" 1, "calt" 1;
        }}
        /* Powerline symbols support */
        .xterm .xterm-char {{
            font-variant-ligatures: contextual;
        }}
        /* Scrollbar styling */
        ::-webkit-scrollbar {{
            width: 8px;
            height: 8px;
        }}
        ::-webkit-scrollbar-track {{
            background: #1e222a;
        }}
        ::-webkit-scrollbar-thumb {{
            background: #4a5568;
            border-radius: 4px;
        }}
        ::-webkit-scrollbar-thumb:hover {{
            background: #718096;
        }}
    </style>
</head>
<body>
    <div id="terminal"></div>
    <script>
        // High-performance terminal configuration
        const term = new Terminal({{
            fontFamily: '{}',
            fontSize: {},
            lineHeight: {},
            cursorBlink: {},
            cursorStyle: '{}',
            scrollback: {},
            allowProposedApi: true,
            allowTransparency: false,
            theme: {{
                background: '#16181d',
                foreground: '#c8d2dc',
                cursor: '#aeafad',
                cursorAccent: '#000000',
                selectionBackground: '#264f78',
                selectionForeground: '#ffffff',
                black: '#0c0c0c',
                red: '#c50f1f',
                green: '#13a10e',
                yellow: '#c19c00',
                blue: '#0037da',
                magenta: '#881798',
                cyan: '#3a96dd',
                white: '#cccccc',
                brightBlack: '#767676',
                brightRed: '#e74856',
                brightGreen: '#16c60c',
                brightYellow: '#f9f1a5',
                brightBlue: '#3b78ff',
                brightMagenta: '#b4009e',
                brightCyan: '#61d6d6',
                brightWhite: '#f2f2f2'
            }},
            // Performance optimizations
            fastScrollModifier: 'alt',
            fastScrollSensitivity: 5,
            smoothScrollDuration: 0, // Disable smooth scroll for better performance
            // True color support
            minimumContrastRatio: 4.5,
            // GPU acceleration
            experimentalCharAtlas: 'dynamic'
        }});

        // Initialize WebGL addon for 60fps rendering
        const webglAddon = new WebglAddon.WebglAddon();
        term.loadAddon(webglAddon);

        // Handle WebGL context loss
        webglAddon.onContextLoss(() => {{
            console.warn('WebGL context lost, attempting recovery...');
            webglAddon.dispose();
            term.loadAddon(new WebglAddon.WebglAddon());
        }});

        // Add other addons
        const fitAddon = new FitAddon.FitAddon();
        term.loadAddon(fitAddon);
        term.loadAddon(new WebLinksAddon.WebLinksAddon());
        term.loadAddon(new SearchAddon.SearchAddon());
        term.loadAddon(new Unicode11Addon.Unicode11Addon());
        term.unicode.activeVersion = '11';

        // Mount terminal
        term.open(document.getElementById('terminal'));
        fitAddon.fit();

        // Expose terminal to window for external control (copy-paste)
        window.term = term;
        window.fitAddon = fitAddon;

        // Performance monitoring
        let frameCount = 0;
        let lastFpsUpdate = performance.now();
        let renderStats = {{ fps: 0, frameTime: 0 }};

        // 60fps render loop
        function renderLoop() {{
            const start = performance.now();

            // Process any pending data
            // (WebGL handles this automatically)

            frameCount++;
            const now = performance.now();
            const elapsed = now - lastFpsUpdate;

            if (elapsed >= 1000) {{
                renderStats.fps = Math.round((frameCount * 1000) / elapsed);
                frameCount = 0;
                lastFpsUpdate = now;

                // Send stats to host
                if (window.chrome && window.chrome.webview) {{
                    window.chrome.webview.postMessage(JSON.stringify({{
                        type: 'renderStats',
                        data: renderStats
                    }}));
                }}
            }}

            const frameTime = performance.now() - start;
            renderStats.frameTime = frameTime;

            // Schedule next frame for 60fps target (16.67ms)
            const targetFrameTime = 16.67;
            const delay = Math.max(0, targetFrameTime - frameTime);

            requestAnimationFrame(() => {{
                setTimeout(renderLoop, delay);
            }});
        }}

        // Start render loop
        requestAnimationFrame(renderLoop);

        // Handle resize
        let resizeTimeout;
        window.addEventListener('resize', () => {{
            clearTimeout(resizeTimeout);
            resizeTimeout = setTimeout(() => {{
                fitAddon.fit();
                if (window.chrome && window.chrome.webview) {{
                    window.chrome.webview.postMessage(JSON.stringify({{
                        type: 'resize',
                        data: {{ cols: term.cols, rows: term.rows }}
                    }}));
                }}
            }}, 100);
        }});

        // Message handling from host
        window.addEventListener('message', (event) => {{
            const msg = typeof event.data === 'string' ? JSON.parse(event.data) : event.data;

            switch (msg.type) {{
                case 'write':
                    term.write(msg.data);
                    break;
                case 'writeln':
                    term.writeln(msg.data);
                    break;
                case 'paste':
                    // Paste text from clipboard
                    if (msg.data) {{
                        term.paste(msg.data);
                    }}
                    break;
                case 'clear':
                    term.clear();
                    break;
                case 'reset':
                    term.reset();
                    break;
                case 'focus':
                    term.focus();
                    break;
                case 'blur':
                    term.blur();
                    break;
                case 'selectAll':
                    term.selectAll();
                    break;
                case 'getSelection':
                    const selection = term.getSelection();
                    if (window.chrome && window.chrome.webview) {{
                        window.chrome.webview.postMessage(JSON.stringify({{
                            type: 'selection',
                            data: selection
                        }}));
                    }}
                    break;
                case 'resize':
                    term.resize(msg.data.cols, msg.data.rows);
                    break;
                case 'scrollToBottom':
                    term.scrollToBottom();
                    break;
                case 'scrollToTop':
                    term.scrollToTop();
                    break;
                case 'scrollLines':
                    term.scrollLines(msg.data.lines);
                    break;
                case 'scrollPages':
                    term.scrollPages(msg.data.pages);
                    break;
                case 'setOption':
                    term.options[msg.data.key] = msg.data.value;
                    break;
                case 'getOptions':
                    if (window.chrome && window.chrome.webview) {{
                        window.chrome.webview.postMessage(JSON.stringify({{
                            type: 'options',
                            data: term.options
                        }}));
                    }}
                    break;
                default:
                    console.log('Unknown message type:', msg.type);
            }}
        }});

        // Input handling
        term.onData((data) => {{
            if (window.chrome && window.chrome.webview) {{
                window.chrome.webview.postMessage(JSON.stringify({{
                    type: 'input',
                    data: data
                }}));
            }}
        }});

        term.onBinary((data) => {{
            if (window.chrome && window.chrome.webview) {{
                window.chrome.webview.postMessage(JSON.stringify({{
                    type: 'binary',
                    data: data
                }}));
            }}
        }});

        // Selection handling
        term.onSelectionChange(() => {{
            const selection = term.getSelection();
            if (window.chrome && window.chrome.webview) {{
                window.chrome.webview.postMessage(JSON.stringify({{
                    type: 'selectionChange',
                    data: selection
                }}));
            }}
        }});

        // Handle big data streaming
        let dataBuffer = [];
        let flushTimeout = null;

        function flushBuffer() {{
            if (dataBuffer.length > 0) {{
                const combined = dataBuffer.join('');
                term.write(combined);
                dataBuffer = [];
            }}
        }}

        window.writeData = function(data) {{
            // Batch writes for better performance with big data
            dataBuffer.push(data);

            if (dataBuffer.length >= 100) {{
                flushBuffer();
            }} else {{
                clearTimeout(flushTimeout);
                flushTimeout = setTimeout(flushBuffer, 1);
            }}
        }};

        // Ready signal
        if (window.chrome && window.chrome.webview) {{
            window.chrome.webview.postMessage(JSON.stringify({{
                type: 'ready',
                data: {{ cols: term.cols, rows: term.rows }}
            }}));
        }}

        console.log('WebGL Terminal initialized with 60fps target');
    </script>
</body>
</html>
        "#,
        if cursor_blink { 530 } else { 0 },
        font_family, font_size, line_height,
        font_family, font_size, line_height, cursor_blink, cursor_style, scrollback,
        )
    }

    /// Write data to terminal
    pub fn write(&mut self, data: &str) {
        let mut buffer = self.buffer.lock();
        // Mark buffer as dirty for re-render
        buffer.mark_dirty();
        // In real implementation, send to WebView
        debug!("Terminal write: {} bytes", data.len());
    }

    /// Write line to terminal
    pub fn writeln(&mut self, data: &str) {
        self.write(data);
        self.write("\r\n");
    }

    /// Clear terminal
    pub fn clear(&mut self) {
        let mut buffer = self.buffer.lock();
        buffer.clear();
    }

    /// Reset terminal
    pub fn reset(&mut self) {
        self.clear();
    }

    /// Resize terminal
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.config.cols = cols;
        self.config.rows = rows;
        let mut buffer = self.buffer.lock();
        buffer.resize(cols, rows);
    }

    /// Get current dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.config.cols, self.config.rows)
    }

    /// Get render stats
    pub fn get_stats(&self) -> RenderStats {
        self.stats.lock().clone()
    }

    /// Get configuration
    pub fn config(&self) -> &TerminalConfig {
        &self.config
    }

    /// Check if should render (60fps throttling)
    pub fn should_render(&self) -> bool {
        let elapsed = self.last_render.elapsed();
        elapsed >= self.frame_time_target
    }

    /// Mark render complete
    pub fn mark_rendered(&mut self) {
        self.last_render = Instant::now();
        let mut stats = self.stats.lock();
        stats.render_calls += 1;
        stats.last_update = Instant::now();
    }

    /// Get the WebView HTML content
    pub fn get_webview_html(&self) -> &str {
        &self.webview_html
    }

    /// Build WebView with this terminal
    pub fn build_webview<'a>(
        &self,
        _builder: WebViewBuilder<'a>,
    ) -> Result<WebView> {
        // WebView creation disabled for now - wry 0.46 API is significantly different
        // TODO: Update to new wry API
        Err(anyhow::anyhow!("WebView building not implemented for wry 0.46"))
    }
}

/// Create default WebGL terminal
pub fn create_default_terminal() -> WebGlTerminal {
    WebGlTerminal::new(TerminalConfig::default())
}

/// Create high-performance terminal for big data streaming
pub fn create_streaming_terminal() -> WebGlTerminal {
    let mut config = TerminalConfig::default();
    config.scrollback_lines = 50_000; // Reduced for streaming performance
    config.target_fps = 60;
    config.vsync = true;
    WebGlTerminal::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_buffer_resize() {
        let mut buffer = TerminalBuffer::new(80, 24);
        buffer.set_cell(0, 0, TerminalCell {
            char: 'X',
            ..Default::default()
        });

        buffer.resize(100, 30);
        assert_eq!(buffer.get_cell(0, 0).unwrap().char, 'X');
    }

    #[test]
    fn test_terminal_buffer_scroll() {
        let mut buffer = TerminalBuffer::new(80, 24);
        buffer.set_cell(0, 0, TerminalCell {
            char: 'A',
            ..Default::default()
        });

        buffer.scroll_up(1);
        // After scroll, row 0 should be empty
        assert_eq!(buffer.get_cell(0, 0).unwrap().char, ' ');
    }

    #[test]
    fn test_html_generation() {
        let config = TerminalConfig::default();
        let html = WebGlTerminal::generate_xterm_html(&config);
        assert!(html.contains("xterm"));
        assert!(html.contains("WebglAddon"));
        assert!(html.contains("60fps"));
    }
}
