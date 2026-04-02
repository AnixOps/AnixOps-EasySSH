//! Animation System
//!
//! Provides smooth, performant animations with:
//! - Reduced motion support (accessibility)
//! - Platform-optimized rendering
//! - Easing curves (spring, smooth, snappy)
//! - Animation presets for common transitions

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Animation configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Enable animations globally
    pub enabled: bool,
    /// Respect reduced motion preference
    pub reduced_motion: bool,
    /// Default duration multiplier (0.5 = half speed, 2.0 = double speed)
    pub speed_multiplier: u32,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reduced_motion: false,
            speed_multiplier: 1,
        }
    }
}

/// Duration presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Duration {
    /// Instant (0ms)
    Instant,
    /// Fast (100ms)
    Fast,
    /// Normal (200ms)
    Normal,
    /// Slow (300ms)
    Slow,
    /// Slower (400ms)
    Slower,
    /// Slowest (500ms)
    Slowest,
}

impl Duration {
    /// Get duration in milliseconds
    pub fn as_millis(&self) -> u64 {
        match self {
            Duration::Instant => 0,
            Duration::Fast => 100,
            Duration::Normal => 200,
            Duration::Slow => 300,
            Duration::Slower => 400,
            Duration::Slowest => 500,
        }
    }

    /// Get duration as std::time::Duration
    pub fn as_std_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.as_millis())
    }
}

/// Easing curves for animations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    /// Linear interpolation
    Linear,
    /// Standard ease
    Ease,
    /// Ease in (accelerating)
    EaseIn,
    /// Ease out (decelerating)
    EaseOut,
    /// Ease in-out (smooth)
    EaseInOut,
    /// Spring (bouncy)
    Spring,
    /// Smooth (ease out with custom curve)
    Smooth,
    /// Snappy (quick response)
    Snappy,
}

impl Easing {
    /// Get CSS cubic-bezier values
    pub fn to_css(&self) -> &'static str {
        match self {
            Easing::Linear => "linear",
            Easing::Ease => "ease",
            Easing::EaseIn => "ease-in",
            Easing::EaseOut => "ease-out",
            Easing::EaseInOut => "ease-in-out",
            Easing::Spring => "cubic-bezier(0.34, 1.56, 0.64, 1)",
            Easing::Smooth => "cubic-bezier(0.23, 1, 0.32, 1)",
            Easing::Snappy => "cubic-bezier(0.25, 0.46, 0.45, 0.94)",
        }
    }

    /// Calculate value at a specific progress (0.0 - 1.0)
    pub fn calculate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::Ease => ease_in_out(t),
            Easing::EaseIn => ease_in(t),
            Easing::EaseOut => ease_out(t),
            Easing::EaseInOut => ease_in_out(t),
            Easing::Spring => spring(t),
            Easing::Smooth => smooth(t),
            Easing::Snappy => snappy(t),
        }
    }
}

fn ease_in(t: f32) -> f32 {
    t * t
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn spring(t: f32) -> f32 {
    // Overshoot spring effect
    let overshoot = 1.0 + 0.1 * (t * std::f32::consts::PI * 3.0).sin();
    t * overshoot
}

fn smooth(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn snappy(t: f32) -> f32 {
    // Quick start, slower end
    t.powf(0.7)
}

/// Animation preset types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Animation {
    /// Fade in
    FadeIn,
    /// Fade out
    FadeOut,
    /// Scale in
    ScaleIn,
    /// Scale out
    ScaleOut,
    /// Slide in from left
    SlideInLeft,
    /// Slide in from right
    SlideInRight,
    /// Slide in from top
    SlideInTop,
    /// Slide in from bottom
    SlideInBottom,
    /// Slide out to left
    SlideOutLeft,
    /// Slide out to right
    SlideOutRight,
    /// Slide out to top
    SlideOutTop,
    /// Slide out to bottom
    SlideOutBottom,
    /// Bounce effect
    Bounce,
    /// Pulse effect
    Pulse,
    /// Spin/rotate
    Spin,
    /// Shimmer loading effect
    Shimmer,
}

impl Animation {
    /// Get default duration for animation
    pub fn default_duration(&self) -> Duration {
        match self {
            Animation::FadeIn | Animation::FadeOut => Duration::Normal,
            Animation::ScaleIn | Animation::ScaleOut => Duration::Normal,
            Animation::SlideInLeft | Animation::SlideInRight => Duration::Slow,
            Animation::SlideInTop | Animation::SlideInBottom => Duration::Slow,
            Animation::SlideOutLeft | Animation::SlideOutRight => Duration::Normal,
            Animation::SlideOutTop | Animation::SlideOutBottom => Duration::Normal,
            Animation::Bounce => Duration::Slow,
            Animation::Pulse => Duration::Slower,
            Animation::Spin => Duration::Slowest,
            Animation::Shimmer => Duration::Slowest,
        }
    }

    /// Get default easing for animation
    pub fn default_easing(&self) -> Easing {
        match self {
            Animation::FadeIn | Animation::FadeOut => Easing::Ease,
            Animation::ScaleIn | Animation::ScaleOut => Easing::Spring,
            Animation::SlideInLeft | Animation::SlideInRight => Easing::Smooth,
            Animation::SlideInTop | Animation::SlideInBottom => Easing::Smooth,
            Animation::SlideOutLeft | Animation::SlideOutRight => Easing::EaseOut,
            Animation::SlideOutTop | Animation::SlideOutBottom => Easing::EaseOut,
            Animation::Bounce => Easing::Spring,
            Animation::Pulse => Easing::Ease,
            Animation::Spin => Easing::Linear,
            Animation::Shimmer => Easing::Linear,
        }
    }

    /// Check if animation should be used with reduced motion
    pub fn is_safe_for_reduced_motion(&self) -> bool {
        // Only non-movement animations are safe
        matches!(self, Animation::FadeIn | Animation::FadeOut | Animation::Pulse)
    }
}

/// Animation value types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimatedValue {
    /// Float value
    Float(f32),
    /// Position (x, y)
    Position { x: f32, y: f32 },
    /// Scale (uniform)
    Scale(f32),
    /// Scale (x, y)
    ScaleXY { x: f32, y: f32 },
    /// Rotation (degrees)
    Rotation(f32),
    /// Opacity (0.0 - 1.0)
    Opacity(f32),
    /// Color (RGBA)
    Color { r: u8, g: u8, b: u8, a: u8 },
}

/// Running animation state
pub struct AnimationState {
    /// Animation type
    pub animation: Animation,
    /// Start time
    pub start_time: Instant,
    /// Duration
    pub duration: Duration,
    /// Easing curve
    pub easing: Easing,
    /// From value
    pub from: AnimatedValue,
    /// To value
    pub to: AnimatedValue,
    /// Whether animation is complete
    pub is_complete: bool,
    /// Callback when complete
    pub on_complete: Option<Box<dyn FnOnce()>>,
}

impl AnimationState {
    /// Create a new animation state
    pub fn new(
        animation: Animation,
        duration: Duration,
        easing: Easing,
        from: AnimatedValue,
        to: AnimatedValue,
    ) -> Self {
        Self {
            animation,
            start_time: Instant::now(),
            duration,
            easing,
            from,
            to,
            is_complete: false,
            on_complete: None,
        }
    }

    /// Get current progress (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed();
        let duration = std::time::Duration::from_millis(self.duration.as_millis());

        if elapsed >= duration {
            1.0
        } else {
            elapsed.as_secs_f32() / duration.as_secs_f32()
        }
    }

    /// Get current value
    pub fn current_value(&self) -> AnimatedValue {
        let t = self.easing.calculate(self.progress());
        interpolate_value(&self.from, &self.to, t)
    }

    /// Update animation state
    pub fn update(&mut self) {
        if !self.is_complete && self.progress() >= 1.0 {
            self.is_complete = true;
            if let Some(callback) = self.on_complete.take() {
                callback();
            }
        }
    }
}

/// Interpolate between two values
fn interpolate_value(from: &AnimatedValue, to: &AnimatedValue, t: f32) -> AnimatedValue {
    match (from, to) {
        (AnimatedValue::Float(a), AnimatedValue::Float(b)) => {
            AnimatedValue::Float(lerp(*a, *b, t))
        }
        (AnimatedValue::Position { x: x1, y: y1 }, AnimatedValue::Position { x: x2, y: y2 }) => {
            AnimatedValue::Position {
                x: lerp(*x1, *x2, t),
                y: lerp(*y1, *y2, t),
            }
        }
        (AnimatedValue::Scale(a), AnimatedValue::Scale(b)) => {
            AnimatedValue::Scale(lerp(*a, *b, t))
        }
        (AnimatedValue::ScaleXY { x: x1, y: y1 }, AnimatedValue::ScaleXY { x: x2, y: y2 }) => {
            AnimatedValue::ScaleXY {
                x: lerp(*x1, *x2, t),
                y: lerp(*y1, *y2, t),
            }
        }
        (AnimatedValue::Rotation(a), AnimatedValue::Rotation(b)) => {
            AnimatedValue::Rotation(lerp(*a, *b, t))
        }
        (AnimatedValue::Opacity(a), AnimatedValue::Opacity(b)) => {
            AnimatedValue::Opacity(lerp(*a, *b, t))
        }
        _ => to.clone(), // Fallback
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Animation manager for running multiple animations
pub struct AnimationManager {
    config: AnimationConfig,
    animations: Vec<AnimationState>,
    reduced_motion: bool,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new(config: &AnimationConfig) -> Self {
        let reduced_motion = config.reduced_motion || is_reduced_motion_preferred();

        Self {
            config: *config,
            animations: Vec::new(),
            reduced_motion,
        }
    }

    /// Start a new animation
    pub fn start(&mut self, mut state: AnimationState) -> usize {
        // Apply reduced motion preference
        if self.reduced_motion && !state.animation.is_safe_for_reduced_motion() {
            // Convert to fade or make instant
            state.duration = Duration::Instant;
        }

        let id = self.animations.len();
        self.animations.push(state);
        id
    }

    /// Create and start a fade in animation
    pub fn fade_in(&mut self, duration: Option<Duration>) -> usize {
        let duration = duration.unwrap_or(Duration::Normal);
        let state = AnimationState::new(
            Animation::FadeIn,
            duration,
            Easing::Ease,
            AnimatedValue::Opacity(0.0),
            AnimatedValue::Opacity(1.0),
        );
        self.start(state)
    }

    /// Create and start a fade out animation
    pub fn fade_out(&mut self, duration: Option<Duration>) -> usize {
        let duration = duration.unwrap_or(Duration::Normal);
        let state = AnimationState::new(
            Animation::FadeOut,
            duration,
            Easing::Ease,
            AnimatedValue::Opacity(1.0),
            AnimatedValue::Opacity(0.0),
        );
        self.start(state)
    }

    /// Create and start a scale in animation
    pub fn scale_in(&mut self, duration: Option<Duration>) -> usize {
        let duration = duration.unwrap_or(Duration::Normal);
        let state = AnimationState::new(
            Animation::ScaleIn,
            duration,
            Easing::Spring,
            AnimatedValue::Scale(0.9),
            AnimatedValue::Scale(1.0),
        );
        self.start(state)
    }

    /// Create and start a slide in animation
    pub fn slide_in(&mut self, direction: SlideDirection, duration: Option<Duration>) -> usize {
        let animation = match direction {
            SlideDirection::Left => Animation::SlideInLeft,
            SlideDirection::Right => Animation::SlideInRight,
            SlideDirection::Top => Animation::SlideInTop,
            SlideDirection::Bottom => Animation::SlideInBottom,
        };

        let from_pos = match direction {
            SlideDirection::Left => AnimatedValue::Position { x: -100.0, y: 0.0 },
            SlideDirection::Right => AnimatedValue::Position { x: 100.0, y: 0.0 },
            SlideDirection::Top => AnimatedValue::Position { x: 0.0, y: -100.0 },
            SlideDirection::Bottom => AnimatedValue::Position { x: 0.0, y: 100.0 },
        };

        let duration = duration.unwrap_or(Duration::Slow);
        let state = AnimationState::new(
            animation,
            duration,
            Easing::Smooth,
            from_pos,
            AnimatedValue::Position { x: 0.0, y: 0.0 },
        );
        self.start(state)
    }

    /// Update all animations
    pub fn update(&mut self) {
        for anim in &mut self.animations {
            anim.update();
        }
        // Remove completed animations
        self.animations.retain(|a| !a.is_complete);
    }

    /// Get animation state by id
    pub fn get(&self, id: usize) -> Option<&AnimationState> {
        self.animations.get(id)
    }

    /// Check if any animations are running
    pub fn has_running(&self) -> bool {
        self.animations.iter().any(|a| !a.is_complete)
    }

    /// Cancel an animation
    pub fn cancel(&mut self, id: usize) {
        if id < self.animations.len() {
            self.animations.remove(id);
        }
    }

    /// Cancel all animations
    pub fn cancel_all(&mut self) {
        self.animations.clear();
    }

    /// Set reduced motion preference
    pub fn set_reduced_motion(&mut self, enabled: bool) {
        self.reduced_motion = enabled;
    }
}

/// Slide direction for animations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideDirection {
    Left,
    Right,
    Top,
    Bottom,
}

/// Check if user prefers reduced motion
fn is_reduced_motion_preferred() -> bool {
    // Platform-specific detection
    // For now, default to false
    false
}

/// CSS animation class generator
pub struct CssAnimationBuilder {
    name: String,
    duration: Duration,
    easing: Easing,
    delay: Duration,
    iterations: IterationCount,
    direction: AnimationDirection,
    fill_mode: FillMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationCount {
    Once,
    Infinite,
    Count(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

impl CssAnimationBuilder {
    /// Create a new CSS animation builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            duration: Duration::Normal,
            easing: Easing::Ease,
            delay: Duration::Instant,
            iterations: IterationCount::Once,
            direction: AnimationDirection::Normal,
            fill_mode: FillMode::Both,
        }
    }

    /// Set duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set easing
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Set delay
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Set iterations
    pub fn infinite(mut self) -> Self {
        self.iterations = IterationCount::Infinite;
        self
    }

    /// Build CSS string
    pub fn build(&self) -> String {
        let iteration = match self.iterations {
            IterationCount::Once => "1",
            IterationCount::Infinite => "infinite",
            IterationCount::Count(n) => &format!("{}", n),
        };

        let direction = match self.direction {
            AnimationDirection::Normal => "normal",
            AnimationDirection::Reverse => "reverse",
            AnimationDirection::Alternate => "alternate",
            AnimationDirection::AlternateReverse => "alternate-reverse",
        };

        let fill_mode = match self.fill_mode {
            FillMode::None => "none",
            FillMode::Forwards => "forwards",
            FillMode::Backwards => "backwards",
            FillMode::Both => "both",
        };

        format!(
            "animation: {} {} {} {} {} {};",
            self.name,
            format!("{}ms", self.duration.as_millis()),
            self.easing.to_css(),
            format!("{}ms", self.delay.as_millis()),
            iteration,
            direction,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_calculations() {
        assert_eq!(Easing::Linear.calculate(0.5), 0.5);
        assert!(Easing::EaseIn.calculate(0.5) < 0.5);
        assert!(Easing::EaseOut.calculate(0.5) > 0.5);
    }

    #[test]
    fn test_animation_durations() {
        assert_eq!(Duration::Instant.as_millis(), 0);
        assert_eq!(Duration::Normal.as_millis(), 200);
        assert_eq!(Duration::Slowest.as_millis(), 500);
    }

    #[test]
    fn test_css_animation_builder() {
        let css = CssAnimationBuilder::new("fadeIn")
            .duration(Duration::Slow)
            .easing(Easing::Spring)
            .infinite()
            .build();

        assert!(css.contains("fadeIn"));
        assert!(css.contains("300ms"));
        assert!(css.contains("infinite"));
    }

    #[test]
    fn test_animation_state() {
        let mut state = AnimationState::new(
            Animation::FadeIn,
            Duration::Instant,
            Easing::Linear,
            AnimatedValue::Opacity(0.0),
            AnimatedValue::Opacity(1.0),
        );

        state.update();
        assert!(state.is_complete);
        assert_eq!(state.current_value(), AnimatedValue::Opacity(1.0));
    }

    #[test]
    fn test_reduced_motion_safety() {
        assert!(Animation::FadeIn.is_safe_for_reduced_motion());
        assert!(!Animation::SlideInLeft.is_safe_for_reduced_motion());
    }
}
