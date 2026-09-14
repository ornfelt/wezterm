//! Neovide style "smear" cursor animation.
//!
//! The cursor rectangle is tracked as four independent corners, each of which
//! continuously chases its counterpart on the real cursor rect. The corners
//! that point away from the direction of travel chase more slowly, so the
//! rectangle stretches into a trail that closes up again once they catch up.
//!
//! The chase is exponential rather than a fixed length tween: each corner moves
//! a fraction of its remaining distance per second. That matters when the
//! cursor keeps moving - a held down key, or a command writing out a screenful
//! of text - because the corners then settle at a constant distance behind the
//! cursor instead of restarting a tween, and falling further behind, on every
//! single step.

use config::SmearCursor;
use mux::pane::PaneId;
use std::time::Instant;
use wezterm_term::StableRowIndex;

/// A quad is made up of four corners; they are always kept in the order
/// top-left, top-right, bottom-right, bottom-left.
pub const SMEAR_CORNER_COUNT: usize = 4;

/// Where each corner sits relative to the centre of the cursor rect,
/// expressed as a fraction of the size of that rect.
const CORNER_OFFSETS: [(f32, f32); SMEAR_CORNER_COUNT] =
    [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];

const MILLIS_PER_SECOND: f32 = 1000.;
/// Guards against a division by zero when duration_ms is configured as 0
const MIN_DURATION_SECONDS: f32 = 0.001;
/// A trail size of exactly 1 would stop the trailing corners from ever arriving
const MAX_TRAIL_SIZE: f32 = 0.95;
/// Floor for the decay rate, so that a movement barely longer than the stop
/// distance still eases rather than stepping straight there. Covers all but
/// `e^-2`, ie 86%, of the distance within `duration_ms`.
const MIN_DECAY: f32 = 2.0;
/// Once every corner is within this fraction of a cell of where it belongs,
/// the animation has visibly finished
const STOP_DISTANCE_CELLS: f32 = 0.1;
/// A movement this long, in cells, gets the full `max_duration_ms`. Anything
/// shorter is scaled between `duration_ms` and that, so a jump across the
/// window does not cover most of its distance in the first frame or two.
const LONG_MOVE_CELLS: f32 = 25.;

/// The area covered by the cursor, in window pixel coordinates
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CursorRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CursorRect {
    fn corners(&self) -> [(f32, f32); SMEAR_CORNER_COUNT] {
        let center_x = self.x + self.width / 2.;
        let center_y = self.y + self.height / 2.;
        let mut corners = [(0., 0.); SMEAR_CORNER_COUNT];
        for (corner, (off_x, off_y)) in corners.iter_mut().zip(CORNER_OFFSETS.iter()) {
            *corner = (
                center_x + off_x * self.width,
                center_y + off_y * self.height,
            );
        }
        corners
    }
}

/// Where the cursor is, plus enough context to tell a movement you made apart
/// from one the pane's own output caused.
#[derive(Clone, Copy)]
pub struct CursorMotion {
    pub rect: CursorRect,
    /// Stable row the cursor sits on. Output pushes this forward as it writes
    /// new lines, whether or not the screen has started scrolling yet.
    pub row: StableRowIndex,
    /// Stable row at the top of the viewport. Changes when the view scrolls,
    /// either because output pushed it or because the user scrolled back.
    pub viewport_top: StableRowIndex,
    /// Full screen applications move the cursor on purpose, several rows at a
    /// time, and never scroll the scrollback while doing it.
    pub alt_screen: bool,
}

pub struct SmearCursorState {
    /// Where each corner of the smear currently is, in window pixel coordinates
    corners: [(f32, f32); SMEAR_CORNER_COUNT],
    /// The rect the cursor occupied the last time we looked
    target: Option<CursorRect>,
    /// Which pane that rect belonged to
    pane: Option<PaneId>,
    /// Cursor row and viewport position as of the previous frame
    last_row: Option<StableRowIndex>,
    last_viewport_top: Option<StableRowIndex>,
    /// How far the cursor itself last moved, in cells. This sets the pace of
    /// the animation. It has to be the cursor's own step rather than how far
    /// the quad still has to go, because the quad's lag is precisely what the
    /// pace controls: feeding it back in makes a slow animation lag further,
    /// which would make it slower still.
    pace_cells: f32,
    /// Unit vector of the most recent cursor movement; decides which corners
    /// are leading and which are trailing
    travel_dir: (f32, f32),
    last_update: Instant,
    animating: bool,
}

impl SmearCursorState {
    pub fn new() -> Self {
        Self {
            corners: [(0., 0.); SMEAR_CORNER_COUNT],
            target: None,
            pane: None,
            last_row: None,
            last_viewport_top: None,
            pace_cells: 0.,
            travel_dir: (0., 0.),
            last_update: Instant::now(),
            animating: false,
        }
    }

    /// Forget about any in-flight animation; used when the cursor isn't
    /// visible, so that it doesn't smear in from wherever it was last seen.
    pub fn reset(&mut self) {
        self.target = None;
        self.pane = None;
        self.last_row = None;
        self.last_viewport_top = None;
        self.animating = false;
    }

    /// Switching pane, tab or window moves the cursor somewhere unrelated,
    /// which isn't a movement worth smearing.
    pub fn note_pane(&mut self, pane_id: PaneId) {
        if self.pane.replace(pane_id) != Some(pane_id) {
            self.target = None;
            self.animating = false;
        }
    }

    /// Advance the animation and report the corners of the smear quad, in
    /// window pixel coordinates, in the order top-left, top-right,
    /// bottom-right, bottom-left. Returns None when there is nothing to draw.
    pub fn update(
        &mut self,
        config: &SmearCursor,
        motion: CursorMotion,
        cell_size: (f32, f32),
        now: Instant,
    ) -> Option<[(f32, f32); SMEAR_CORNER_COUNT]> {
        let target = motion.rect;
        let destination = target.corners();

        // Both of these are sampled every frame, whatever we end up doing with
        // them, so that they never go stale across an alt screen switch
        let previous_top = self.last_viewport_top.replace(motion.viewport_top);
        let previous_row = self.last_row.replace(motion.row);

        let previous_target = match self.target.replace(target) {
            Some(previous) => previous,
            None => {
                // First sighting of the cursor; there is nothing to smear from
                self.snap(destination, now);
                return None;
            }
        };

        // New output pushes the cursor down the scrollback a row at a time,
        // which on the main screen accounts for essentially every vertical
        // move the cursor makes on its own.
        let pushed_by_output = previous_row.map_or(false, |previous| motion.row > previous);
        // The view moving under the cursor is not the cursor moving
        let view_scrolled = previous_top != Some(motion.viewport_top);
        let output_burst = !config.smear_output && !motion.alt_screen && pushed_by_output;

        if view_scrolled && !pushed_by_output {
            // The user scrolled the view out from under the cursor
            self.snap(destination, now);
            return None;
        }

        // While output streams out, follow it down the screen but hold the
        // smear where it is horizontally. The vertical motion is what reads as
        // the cursor being carried along by the output; it is the horizontal
        // component that makes a zigzag of it, because each frame the cursor
        // lands at the end of one line and then back at column 0 of the next.
        // When the output stops, the real column is picked up again and the
        // smear settles across to it.
        let (destination, aim) = if output_burst {
            let parked_x = center(&self.corners).0 - target.width / 2.;
            let aim = CursorRect {
                x: parked_x,
                ..target
            };
            (aim.corners(), aim)
        } else {
            (destination, target)
        };

        // How far the quad still has to go. Measuring from the quad rather than
        // from the cursor's previous position is what lets an interrupted
        // burst, an in-flight smear and an ordinary single step all be treated
        // alike.
        let travel = (
            (aim.x + aim.width / 2.) - center(&self.corners).0,
            (aim.y + aim.height / 2.) - center(&self.corners).1,
        );

        if !self.animating {
            let stop_distance = STOP_DISTANCE_CELLS * cell_size.0.min(cell_size.1);
            if travel.0.hypot(travel.1) <= stop_distance {
                // Already where it belongs
                self.last_update = now;
                return None;
            }
            if !self.is_worth_animating(config, travel, cell_size) {
                self.snap(destination, now);
                return None;
            }
            self.animating = true;
        }

        let dt = now
            .saturating_duration_since(self.last_update)
            .as_secs_f32();
        self.last_update = now;

        // Pace the animation by how far the cursor itself just moved. A single
        // long jump earns a longer animation; a key held down, which steps a
        // cell at a time, stays brisk however far behind the quad happens to be.
        let step = (
            (target.x - previous_target.x) / cell_size.0.max(1.),
            (target.y - previous_target.y) / cell_size.1.max(1.),
        );
        // A burst is only ever chased vertically, so its sideways hop between
        // the end of one line and the start of the next must not set the pace
        let step = if output_burst {
            step.1.abs()
        } else {
            step.0.hypot(step.1)
        };
        if step > 0. {
            self.pace_cells = step;
        }

        let reach = (self.pace_cells / LONG_MOVE_CELLS).clamp(0., 1.);
        let short = config.duration_ms as f32;
        let long = (config.max_duration_ms as f32).max(short);
        let duration =
            ((short + (long - short) * reach) / MILLIS_PER_SECOND).max(MIN_DURATION_SECONDS);

        // An exponential chase never mathematically arrives, so pick the decay
        // rate that puts the trailing corner within the stop distance after
        // exactly `duration`. A fixed rate would instead leave a long smear
        // visible for about twice that, since it has that much further to decay.
        let stop_distance = STOP_DISTANCE_CELLS * cell_size.0.min(cell_size.1);
        let span = self.pace_cells * cell_size.0.min(cell_size.1);
        let trailing_rate = (span / stop_distance).max(1.).ln().max(MIN_DECAY) / duration;
        let trail = config.trail_size.clamp(0., MAX_TRAIL_SIZE);
        // The leading corners keep up with the cursor; the trailing ones are
        // slower by exactly the configured trail size, which is what opens the
        // quad up into a smear
        let leading_rate = trailing_rate / (1. - trail);

        // Which way the smear is headed. Taking this from the quad's remaining
        // travel rather than from the last individual cursor step keeps it
        // steady, because the quad's centre lags behind the cursor.
        let heading = normalize(travel);
        if heading != (0., 0.) {
            self.travel_dir = heading;
        }

        // Corners pointing away from the direction of travel are the ones that
        // lag behind and stretch the cursor into a smear. Scaling by the
        // largest lag of the four keeps the smear the same length whichever way
        // the cursor went: for a straight left/right or up/down move no corner
        // opposes the travel by more than 45 degrees, so without this the
        // trailing corners would only ever be held back by about 70% of
        // `trail_size`.
        let lags = CORNER_OFFSETS.map(|offset| -dot(self.travel_dir, normalize(offset)));
        let max_lag = lags.iter().cloned().fold(0., f32::max);

        let mut still_animating = false;

        for ((corner, dest), raw_lag) in self
            .corners
            .iter_mut()
            .zip(destination.iter())
            .zip(lags.iter())
        {
            let lag = if max_lag > 0. {
                (raw_lag / max_lag).clamp(0., 1.)
            } else {
                0.
            };
            let rate = leading_rate + (trailing_rate - leading_rate) * lag;

            // Fraction of the remaining distance to cover in this frame. Going
            // through exp() rather than a fixed step per frame keeps the speed
            // the same whatever the frame rate happens to be.
            let advance = 1. - (-rate * dt).exp();
            corner.0 += (dest.0 - corner.0) * advance;
            corner.1 += (dest.1 - corner.1) * advance;

            if distance(*corner, *dest) > stop_distance {
                still_animating = true;
            } else {
                *corner = *dest;
            }
        }

        if !still_animating {
            self.animating = false;
            self.pace_cells = 0.;
            return None;
        }

        Some(self.corners)
    }

    /// Is the next frame going to look any different from this one?
    pub fn is_animating(&self) -> bool {
        self.animating
    }

    fn is_worth_animating(
        &self,
        config: &SmearCursor,
        travel: (f32, f32),
        cell_size: (f32, f32),
    ) -> bool {
        if cell_size.0 <= 0. || cell_size.1 <= 0. {
            return false;
        }
        let cells = (travel.0 / cell_size.0, travel.1 / cell_size.1);
        let distance = (cells.0 * cells.0 + cells.1 * cells.1).sqrt();

        if distance < config.min_distance_cells {
            return false;
        }
        if config.max_distance_cells > 0. && distance > config.max_distance_cells {
            return false;
        }
        true
    }

    fn snap(&mut self, destination: [(f32, f32); SMEAR_CORNER_COUNT], now: Instant) {
        self.corners = destination;
        self.animating = false;
        self.pace_cells = 0.;
        self.last_update = now;
    }
}

fn normalize(v: (f32, f32)) -> (f32, f32) {
    let length = (v.0 * v.0 + v.1 * v.1).sqrt();
    if length > 0. {
        (v.0 / length, v.1 / length)
    } else {
        (0., 0.)
    }
}

fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    (dx * dx + dy * dy).sqrt()
}

fn center(corners: &[(f32, f32); SMEAR_CORNER_COUNT]) -> (f32, f32) {
    let count = SMEAR_CORNER_COUNT as f32;
    let sum = corners
        .iter()
        .fold((0., 0.), |acc, c| (acc.0 + c.0, acc.1 + c.1));
    (sum.0 / count, sum.1 / count)
}
