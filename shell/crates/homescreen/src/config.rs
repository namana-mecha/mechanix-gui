#![allow(dead_code, unused)]

/// Configuration for the homescreen application
///
/// This struct centralizes all constants and configuration values used throughout
/// the homescreen.
#[derive(Clone, Debug)]
pub struct HomescreenConfig {
    /// Window configuration
    pub window: WindowConfig,

    /// Grid layout configuration
    pub grid: GridConfig,

    /// Animation configuration
    pub animation: AnimationConfig,

    /// Interaction configuration
    pub interaction: InteractionConfig,

    /// Visual styling configuration
    pub visual: VisualConfig,
}

#[derive(Clone, Debug)]
pub struct WindowConfig {
    /// Default window width in pixels
    pub width: f32,

    /// Default window height in pixels
    pub height: f32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 540.0,
            height: 540.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GridConfig {
    /// Number of columns in the widget grid
    pub cols: usize,

    /// Number of rows in the widget grid
    pub rows: usize,

    /// Grid padding as a percentage of window width (0.0 to 1.0)
    pub padding_percent: f32,

    /// Gap between cells as a percentage of window width (0.0 to 1.0)
    pub gap_percent: f32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            cols: 4,
            rows: 4,
            padding_percent: 0.05,
            gap_percent: 0.01,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationConfig {
    /// Page transition animation duration
    pub page_transition: std::time::Duration,

    /// Widget return animation duration (when dropped back to original position)
    pub widget_return: std::time::Duration,

    /// Maximum delta time for animation updates to prevent jumps (in seconds)
    pub max_delta_time: f32,

    /// Minimum distance threshold to consider animation complete (in pixels)
    pub completion_threshold: f32,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            page_transition: std::time::Duration::from_millis(300),
            widget_return: std::time::Duration::from_millis(300),
            max_delta_time: 0.1,
            completion_threshold: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InteractionConfig {
    /// Distance threshold to start page drag (in pixels)
    pub drag_threshold: f32,

    /// Duration to hold before starting widget drag (in seconds)
    pub hold_duration: f32,

    /// Maximum movement allowed during hold (in pixels)
    pub hold_movement_threshold: f32,

    /// Distance from edge to trigger page switch when dragging widgets (in pixels)
    pub edge_trigger_threshold: f32,

    /// Duration to hold at edge before switching pages (in milliseconds)
    pub edge_hold_duration: u64,

    /// Cooldown between page switches (in milliseconds)
    pub page_switch_cooldown: u64,

    /// Percentage of window width to swipe before page changes (0.0 to 1.0)
    pub page_swipe_threshold: f32,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            drag_threshold: 10.0,
            hold_duration: 1.0,
            hold_movement_threshold: 10.0,
            edge_trigger_threshold: 50.0,
            edge_hold_duration: 500,
            page_switch_cooldown: 500,
            page_swipe_threshold: 0.25,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VisualConfig {
    /// Gap between pages in pixels
    pub page_gap: f32,

    /// Background color of the homescreen (hex format)
    pub background_color: u32,

    /// Active page indicator color (hex format)
    pub indicator_active_color: u32,

    /// Inactive page indicator color (hex format with alpha)
    pub indicator_inactive_color: u32,

    /// Page indicator dot size in pixels
    pub indicator_dot_size: f32,

    /// Gap between page indicator dots in pixels
    pub indicator_dot_gap: f32,

    /// Default widget background color (hex format)
    pub default_widget_color: u32,

    /// Debug info position offset from edges (in pixels)
    pub debug_info_offset: f32,

    /// Debug info text size (in pixels)
    pub debug_info_text_size: f32,

    /// Debug info text color (hex format with alpha)
    pub debug_info_text_color: u32,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            page_gap: 40.0,
            background_color: 0x1a1a1a,
            indicator_active_color: 0xFFFFFF,
            indicator_inactive_color: 0xFFFFFF66,
            indicator_dot_size: 8.0,
            indicator_dot_gap: 8.0,
            default_widget_color: 0xFF69B4,
            debug_info_offset: 20.0,
            debug_info_text_size: 12.0,
            debug_info_text_color: 0xFFFFFF88,
        }
    }
}

impl Default for HomescreenConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            grid: GridConfig::default(),
            animation: AnimationConfig::default(),
            interaction: InteractionConfig::default(),
            visual: VisualConfig::default(),
        }
    }
}

impl HomescreenConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
}
