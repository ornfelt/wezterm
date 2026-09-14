use crate::bell::EasingFunction;
use crate::color::RgbaColor;
use wezterm_dynamic::{FromDynamic, ToDynamic};

/// How long the slowest part of the smear takes to catch up with the cursor
const DEFAULT_DURATION_MS: u64 = 130;
/// How far the trailing corners lag behind the leading ones.
/// 0 moves the whole cursor rigidly, values closer to 1 stretch it out further.
const DEFAULT_TRAIL_SIZE: f32 = 0.7;
/// Fully opaque, matching the way the regular cursor is drawn
const DEFAULT_OPACITY: f32 = 1.0;
/// Cursor movements shorter than this, measured in cells, are not worth animating
const DEFAULT_MIN_DISTANCE_CELLS: f32 = 0.5;
/// A distance of 0 cells means "no upper limit"
const DEFAULT_MAX_DISTANCE_CELLS: f32 = 0.0;

fn default_duration_ms() -> u64 {
    DEFAULT_DURATION_MS
}

fn default_trail_size() -> f32 {
    DEFAULT_TRAIL_SIZE
}

fn default_opacity() -> f32 {
    DEFAULT_OPACITY
}

fn default_min_distance_cells() -> f32 {
    DEFAULT_MIN_DISTANCE_CELLS
}

fn default_max_distance_cells() -> f32 {
    DEFAULT_MAX_DISTANCE_CELLS
}

fn default_easing() -> EasingFunction {
    EasingFunction::EaseOut
}

/// Neovide style "smear" cursor: when the cursor moves, the quad that makes up
/// the cursor is stretched towards its previous position and then snaps back
/// into shape, leaving a short lived trail behind it.
#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct SmearCursor {
    /// The smear is not drawn at all unless this is turned on
    #[dynamic(default)]
    pub enabled: bool,

    /// Duration of the whole animation, in milliseconds.
    /// Smaller values make the smear faster and shorter lived.
    #[dynamic(default = "default_duration_ms")]
    pub duration_ms: u64,

    /// How much the trailing corners lag behind the leading ones, in the
    /// range 0..1. Larger values produce a longer smear.
    #[dynamic(default = "default_trail_size")]
    pub trail_size: f32,

    /// The easing applied to each corner as it travels to its destination
    #[dynamic(default = "default_easing")]
    pub easing: EasingFunction,

    /// Alpha applied to the smear colour, in the range 0..1
    #[dynamic(default = "default_opacity")]
    pub opacity: f32,

    /// Colour of the smear. Defaults to the cursor colour from the palette.
    #[dynamic(default)]
    pub color: Option<RgbaColor>,

    /// Movements shorter than this many cells snap instead of animating
    #[dynamic(default = "default_min_distance_cells")]
    pub min_distance_cells: f32,

    /// Movements longer than this many cells snap instead of animating.
    /// 0 disables the limit.
    #[dynamic(default = "default_max_distance_cells")]
    pub max_distance_cells: f32,

    /// Draw the smear over the text rather than behind it
    #[dynamic(default)]
    pub above_text: bool,
}

impl Default for SmearCursor {
    fn default() -> Self {
        Self {
            enabled: false,
            duration_ms: default_duration_ms(),
            trail_size: default_trail_size(),
            easing: default_easing(),
            opacity: default_opacity(),
            color: None,
            min_distance_cells: default_min_distance_cells(),
            max_distance_cells: default_max_distance_cells(),
            above_text: false,
        }
    }
}
