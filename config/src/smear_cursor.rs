use crate::color::RgbaColor;
use wezterm_dynamic::{FromDynamic, ToDynamic};

/// How long the slowest part of the smear takes to catch up with the cursor
const DEFAULT_DURATION_MS: u64 = 130;
/// How long a movement that crosses the window is allowed to take
const DEFAULT_MAX_DURATION_MS: u64 = 250;
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

fn default_max_duration_ms() -> u64 {
    DEFAULT_MAX_DURATION_MS
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

/// Neovide style "smear" cursor: when the cursor moves, the quad that makes up
/// the cursor is stretched towards its previous position and then snaps back
/// into shape, leaving a short lived trail behind it.
#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct SmearCursor {
    /// The smear is not drawn at all unless this is turned on
    #[dynamic(default)]
    pub enabled: bool,

    /// How long, in milliseconds, a short movement takes to close the smear
    /// back up. Smaller values make the smear faster and shorter lived.
    #[dynamic(default = "default_duration_ms")]
    pub duration_ms: u64,

    /// How long, in milliseconds, a screen-crossing movement is allowed to
    /// take. The duration scales with the distance between these two, so that
    /// a long jump does not cross the whole window in a couple of frames the
    /// way it would if every movement took `duration_ms`.
    #[dynamic(default = "default_max_duration_ms")]
    pub max_duration_ms: u64,

    /// How much the trailing corners lag behind the leading ones, in the
    /// range 0..1. Larger values produce a longer smear.
    #[dynamic(default = "default_trail_size")]
    pub trail_size: f32,

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

    /// Follow the exact cursor position while the pane is printing. Off by
    /// default, because a command printing a screenful of text walks the cursor
    /// from the end of one line to the start of the next once per frame, and
    /// following that horizontal bounce reads as a zigzag. With it off the
    /// smear still follows the output down the screen, but holds its horizontal
    /// position until the output stops and then settles across to the real
    /// column. Full screen applications drive the cursor deliberately and are
    /// unaffected either way.
    #[dynamic(default)]
    pub smear_output: bool,
}

impl Default for SmearCursor {
    fn default() -> Self {
        Self {
            enabled: false,
            duration_ms: default_duration_ms(),
            max_duration_ms: default_max_duration_ms(),
            trail_size: default_trail_size(),
            opacity: default_opacity(),
            color: None,
            min_distance_cells: default_min_distance_cells(),
            max_distance_cells: default_max_distance_cells(),
            above_text: false,
            smear_output: false,
        }
    }
}
