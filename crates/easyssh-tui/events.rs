//! Event Handling
//!
//! This module handles terminal events including:
//! - Keyboard input
//! - Mouse events
//! - Timer ticks
//! - Terminal resize events

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
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
    /// Terminal resize
    Resize(u16, u16),
}

/// Event handler that wraps crossterm event processing
pub struct EventHandler {
    /// Event receiver channel
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate_ms: u64) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Spawn event processing task
        tokio::spawn(async move {
            let tick_rate = Duration::from_millis(tick_rate_ms);
            let mut last_tick = tokio::time::Instant::now();

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
                    last_tick = tokio::time::Instant::now();
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

        Self { receiver }
    }

    /// Get the next event
    pub async fn next(&mut self) -> crate::app::AppResult<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_else(|| "Event channel closed".into())
    }
}
