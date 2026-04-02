//! Event Handling
//!
//! This module handles terminal events including:
//! - Keyboard input
//! - Mouse events (enhanced with clicks, scroll, double-click)
//! - Timer ticks
//! - Terminal resize events
//!
//! Supports advanced mouse interactions like double-click for connect.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub type Key = KeyEvent;

/// Application events
#[derive(Clone, Debug)]
pub enum Event {
    /// Timer tick for periodic updates
    Tick,
    /// Key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Mouse double-click event (synthesized)
    MouseDoubleClick { x: u16, y: u16, button: crossterm::event::MouseButton },
    /// Terminal resize
    Resize(u16, u16),
}

/// Event handler that wraps crossterm event processing
pub struct EventHandler {
    /// Event receiver channel
    receiver: mpsc::UnboundedReceiver<Event>,
    /// Last mouse click for double-click detection
    _last_click: Option<(Instant, u16, u16)>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate_ms: u64) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let tick_rate = Duration::from_millis(tick_rate_ms);
        let mut last_tick = Instant::now();
        let mut last_click: Option<(Instant, u16, u16)> = None;
        let double_click_threshold = Duration::from_millis(300);

        // Spawn event processing task
        tokio::spawn(async move {
            loop {
                // Calculate time until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::from_secs(0));

                // Check if we need to send a tick event
                if last_tick.elapsed() >= tick_rate {
                    if sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }

                // Poll for events with timeout
                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            // Skip release events
                            if key.kind == event::KeyEventKind::Press {
                                if sender.send(Event::Key(key)).is_err() {
                                    break;
                                }
                            }
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            // Handle double-click detection for left button
                            use crossterm::event::{MouseButton, MouseEventKind};

                            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                                let now = Instant::now();
                                if let Some((last_time, last_x, last_y)) = last_click {
                                    if now.duration_since(last_time) < double_click_threshold
                                        && mouse.column == last_x
                                        && mouse.row == last_y
                                    {
                                        // Double-click detected
                                        if sender.send(Event::MouseDoubleClick {
                                            x: mouse.column,
                                            y: mouse.row,
                                            button: MouseButton::Left,
                                        }).is_err() {
                                            break;
                                        }
                                        last_click = None; // Reset to prevent triple-click issues
                                        continue;
                                    }
                                }
                                last_click = Some((now, mouse.column, mouse.row));
                            }

                            if sender.send(Event::Mouse(mouse)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if sender.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        Self {
            receiver,
            _last_click: None,
        }
    }

    /// Get the next event
    pub async fn next(&mut self) -> crate::app::AppResult<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_else(|| "Event channel closed".into())
    }
}

