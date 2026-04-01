#![allow(dead_code)]

//! WebGL Terminal Manager - High-Performance Integration
//!
//! Manages WebGL terminal lifecycle and integrates with SSH sessions
//! for 60fps terminal rendering. Includes clipboard support for copy-paste.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tracing::{debug, info, trace};

use crate::terminal::{
    EguiWebGlTerminal, WebGlTerminalBuilder, TerminalMessage,
    StreamingProcessor, RenderStats, TerminalConfig, ColorSupport
};

/// Manages multiple WebGL terminal sessions with clipboard support
pub struct WebGlTerminalManager {
    /// Active terminal sessions by ID
    terminals: HashMap<String, Arc<Mutex<EguiWebGlTerminal>>>,
    /// Streaming processors for each session
    streamers: HashMap<String, StreamingProcessor>,
    /// Current active terminal
    active_terminal: Option<String>,
    /// Global render stats
    stats: RenderStats,
    /// Last stats update
    last_stats_update: Instant,
}

impl WebGlTerminalManager {
    /// Create new terminal manager
    pub fn new() -> Self {
        info!("Initializing WebGL Terminal Manager (60fps target) with clipboard support");

        Self {
            terminals: HashMap::new(),
            streamers: HashMap::new(),
            active_terminal: None,
            stats: RenderStats::default(),
            last_stats_update: Instant::now(),
        }
    }

    /// Create new terminal session
    pub fn create_session(&mut self, session_id: &str, title: &str) -> Arc<Mutex<EguiWebGlTerminal>> {
        info!("Creating WebGL terminal session: {} (with clipboard)", session_id);

        // Create high-performance terminal with True Color support
        let config = TerminalConfig {
            color_support: ColorSupport::TrueColor,
            cols: 120,
            rows: 40,
            scrollback_lines: 50_000,
            target_fps: 60,
            vsync: true,
            webgl2: true,
            ..Default::default()
        };

        let terminal = WebGlTerminalBuilder::with_config(config)
            .font_family("JetBrains Mono, Cascadia Code, Consolas, monospace")
            .font_size(14.0)
            .cursor_blink(true)
            .scrollback(50_000)
            .target_fps(60)
            .dimensions(120, 40)
            .build();

        // Create streaming processor for this session
        let mut processor = StreamingProcessor::new();
        processor.start();

        // Store references
        self.terminals.insert(session_id.to_string(), terminal.clone());
        self.streamers.insert(session_id.to_string(), processor);

        info!("WebGL terminal session created: {} ({}) with copy-paste support", session_id, title);
        terminal
    }

    /// Get terminal by session ID
    pub fn get_terminal(&self, session_id: &str) -> Option<Arc<Mutex<EguiWebGlTerminal>>> {
        self.terminals.get(session_id).cloned()
    }

    /// Write data to terminal
    pub fn write(&mut self, session_id: &str, data: &str) {
        // Use streaming processor for big data
        if let Some(processor) = self.streamers.get_mut(session_id) {
            let bytes = data.as_bytes();

            if bytes.len() > 8192 {
                // Use streaming for big data
                if let Err(e) = processor.push(bytes) {
                    trace!("Streaming backpressure: {}", e);
                }
            }
        }

        // Also write directly to terminal
        if let Some(terminal) = self.terminals.get(session_id) {
            if let Ok(mut term) = terminal.lock() {
                term.write(data);
            }
        }
    }

    /// Write line to terminal
    pub fn writeln(&mut self, session_id: &str, data: &str) {
        self.write(session_id, data);
        self.write(session_id, "\r\n");
    }

    /// Clear terminal
    pub fn clear(&mut self, session_id: &str) {
        if let Some(terminal) = self.terminals.get(session_id) {
            if let Ok(mut term) = terminal.lock() {
                term.clear();
            }
        }
    }

    /// Reset terminal
    pub fn reset(&mut self, session_id: &str) {
        if let Some(terminal) = self.terminals.get(session_id) {
            if let Ok(mut term) = terminal.lock() {
                term.reset();
            }
        }
    }

    /// Focus terminal
    pub fn focus(&mut self, session_id: &str) {
        self.active_terminal = Some(session_id.to_string());

        if let Some(terminal) = self.terminals.get(session_id) {
            if let Ok(mut term) = terminal.lock() {
                term.focus();
            }
        }
    }

    /// Get active terminal
    pub fn get_active(&self) -> Option<Arc<Mutex<EguiWebGlTerminal>>> {
        self.active_terminal.as_ref()
            .and_then(|id| self.terminals.get(id).cloned())
    }

    /// Get active session ID
    pub fn get_active_session(&self) -> Option<&str> {
        self.active_terminal.as_deref()
    }

    /// Copy selection from active terminal to clipboard
    pub fn copy_selection(&self, session_id: Option<&str>) -> Result<(), String> {
        let id = session_id.or_else(|| self.get_active_session())
            .ok_or("No active terminal session")?;

        if let Some(terminal) = self.terminals.get(id) {
            if let Ok(mut term) = terminal.lock() {
                term.copy_selection()
            } else {
                Err("Failed to lock terminal".to_string())
            }
        } else {
            Err(format!("Terminal session not found: {}", id))
        }
    }

    /// Paste from clipboard to active terminal
    pub fn paste_to_terminal(&mut self, session_id: Option<&str>) -> Result<(), String> {
        let id = session_id.or_else(|| self.get_active_session())
            .ok_or("No active terminal session")?;

        if let Some(terminal) = self.terminals.get(id) {
            if let Ok(mut term) = terminal.lock() {
                term.paste_from_clipboard()
            } else {
                Err("Failed to lock terminal".to_string())
            }
        } else {
            Err(format!("Terminal session not found: {}", id))
        }
    }

    /// Select all text in terminal
    pub fn select_all(&mut self, session_id: Option<&str>) -> Result<(), String> {
        let id = session_id.or_else(|| self.get_active_session())
            .ok_or("No active terminal session")?;

        if let Some(terminal) = self.terminals.get(id) {
            if let Ok(mut term) = terminal.lock() {
                term.select_all();
                Ok(())
            } else {
                Err("Failed to lock terminal".to_string())
            }
        } else {
            Err(format!("Terminal session not found: {}", id))
        }
    }

    /// Poll streaming output for session
    pub fn poll_streaming_output(&mut self, session_id: &str) -> Vec<String> {
        let mut output = Vec::new();

        if let Some(processor) = self.streamers.get_mut(session_id) {
            while let Some(data) = processor.recv() {
                output.push(data);
            }
        }

        output
    }

    /// Get render stats
    pub fn update_stats(&mut self) {
        if self.last_stats_update.elapsed().as_secs() >= 1 {
            // Aggregate stats from all terminals
            let mut total_fps = 0.0;
            let mut count = 0;

            for terminal in self.terminals.values() {
                if let Ok(term) = terminal.lock() {
                    let stats = term.render_stats();
                    total_fps += stats.fps;
                    count += 1;
                }
            }

            if count > 0 {
                self.stats.fps = total_fps / count as f32;
            }

            self.last_stats_update = Instant::now();

            debug!("WebGL Terminal aggregate FPS: {:.1}", self.stats.fps);
        }
    }

    /// Get current stats
    pub fn stats(&self) -> &RenderStats {
        &self.stats
    }

    /// Remove terminal session
    pub fn remove_session(&mut self, session_id: &str) {
        self.terminals.remove(session_id);
        self.streamers.remove(session_id);

        if self.active_terminal.as_deref() == Some(session_id) {
            self.active_terminal = None;
        }

        info!("Removed WebGL terminal session: {}", session_id);
    }

    /// Get all session IDs
    pub fn session_ids(&self) -> Vec<String> {
        self.terminals.keys().cloned().collect()
    }

    /// Check if session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        self.terminals.contains_key(session_id)
    }

    /// Process all terminal messages
    pub fn process_messages(&mut self) {
        for (session_id, terminal) in &self.terminals {
            if let Ok(mut term) = terminal.lock() {
                let messages = term.poll_messages();

                for msg in messages {
                    match msg {
                        TerminalMessage::Input(data) => {
                            trace!("Terminal input from {}: {} bytes", session_id, data.len());
                        }
                        TerminalMessage::SelectionChange(text) => {
                            debug!("Selection changed in {}: {} chars", session_id, text.len());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Check if clipboard is available
    pub fn clipboard_available(&self) -> bool {
        if let Some(terminal) = self.get_active() {
            if let Ok(term) = terminal.lock() {
                return term.clipboard_available();
            }
        }
        false
    }
}

impl Default for WebGlTerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WebGlTerminalManager {
    fn drop(&mut self) {
        info!("Shutting down WebGL Terminal Manager");

        // Stop all streaming processors
        for (id, processor) in &mut self.streamers {
            info!("Stopping streamer for session: {}", id);
            processor.stop();
        }

        self.terminals.clear();
        self.streamers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_manager_creation() {
        let manager = WebGlTerminalManager::new();
        assert!(manager.session_ids().is_empty());
    }

    #[test]
    fn test_session_lifecycle() {
        let mut manager = WebGlTerminalManager::new();

        // Create session
        let term = manager.create_session("test-1", "Test Terminal");
        assert!(manager.has_session("test-1"));

        // Write to it
        manager.write("test-1", "Hello, World!");

        // Remove it
        manager.remove_session("test-1");
        assert!(!manager.has_session("test-1"));
    }
}
