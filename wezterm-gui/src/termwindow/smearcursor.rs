//! Neovide style "smear" cursor animation.
//!
//! The cursor rectangle is tracked as four independent corners. When the cursor
//! moves, every corner starts travelling towards its new home, but the corners
//! that point away from the direction of travel are held back, which stretches
//! the rectangle into a trail that snaps shut once they catch up.

use config::SmearCursor;
use mux::pane::PaneId;
use std::time::Instant;

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
const MAX_TRAIL_SIZE: f32 = 0.99;

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

#[derive(Clone, Copy)]
struct Corner {
    /// Where this corner was when the current animation started
    start: (f32, f32),
    current: (f32, f32),
    /// Progress from `start` to the destination corner, in the range 0..=1
    t: f32,
    /// How quickly this corner advances relative to the slowest corner
    speed: f32,
}

impl Corner {
    fn at(pos: (f32, f32)) -> Self {
        Self {
            start: pos,
            current: pos,
            t: 1.,
            speed: 1.,
        }
    }
}

pub struct SmearCursorState {
    corners: [Corner; SMEAR_CORNER_COUNT],
    /// The rect the cursor occupied the last time we looked
    target: Option<CursorRect>,
    /// Which pane that rect belonged to
    pane: Option<PaneId>,
    last_update: Instant,
    animating: bool,
}

impl SmearCursorState {
    pub fn new() -> Self {
        Self {
            corners: [Corner::at((0., 0.)); SMEAR_CORNER_COUNT],
            target: None,
            pane: None,
            last_update: Instant::now(),
            animating: false,
        }
    }

    /// Forget about any in-flight animation; used when the cursor isn't
    /// visible, so that it doesn't smear in from wherever it was last seen.
    pub fn reset(&mut self) {
        self.target = None;
        self.pane = None;
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
        target: CursorRect,
        cell_size: (f32, f32),
        now: Instant,
    ) -> Option<[(f32, f32); SMEAR_CORNER_COUNT]> {
        let destination = target.corners();

        match self.target.replace(target) {
            None => {
                // First sighting of the cursor; there is nothing to smear from
                self.snap(destination, now);
                return None;
            }
            Some(prev) if prev != target => {
                let travel = (target.x - prev.x, target.y - prev.y);
                if self.is_worth_animating(config, travel, cell_size) {
                    self.restart(config, travel);
                } else {
                    self.snap(destination, now);
                    return None;
                }
            }
            Some(_) => {}
        }

        if !self.animating {
            self.last_update = now;
            return None;
        }

        let dt = now
            .saturating_duration_since(self.last_update)
            .as_secs_f32();
        self.last_update = now;

        let duration = (config.duration_ms as f32 / MILLIS_PER_SECOND).max(MIN_DURATION_SECONDS);

        let mut still_animating = false;
        for (corner, dest) in self.corners.iter_mut().zip(destination.iter()) {
            corner.t = (corner.t + (dt / duration) * corner.speed).min(1.);
            let eased = config.easing.evaluate_at_position(corner.t);
            corner.current = (
                corner.start.0 + (dest.0 - corner.start.0) * eased,
                corner.start.1 + (dest.1 - corner.start.1) * eased,
            );
            if corner.t < 1. {
                still_animating = true;
            }
        }
        self.animating = still_animating;

        Some(self.positions())
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
        let cells_x = travel.0 / cell_size.0;
        let cells_y = travel.1 / cell_size.1;
        let distance = (cells_x * cells_x + cells_y * cells_y).sqrt();

        if distance < config.min_distance_cells {
            return false;
        }
        if config.max_distance_cells > 0. && distance > config.max_distance_cells {
            return false;
        }
        true
    }

    /// Begin a new animation from wherever the corners currently are
    fn restart(&mut self, config: &SmearCursor, travel: (f32, f32)) {
        let travel_length = (travel.0 * travel.0 + travel.1 * travel.1).sqrt();
        let travel_dir = if travel_length > 0. {
            (travel.0 / travel_length, travel.1 / travel_length)
        } else {
            (0., 0.)
        };
        let trail = config.trail_size.clamp(0., MAX_TRAIL_SIZE);

        let mut slowest = f32::MAX;
        for (corner, (off_x, off_y)) in self.corners.iter_mut().zip(CORNER_OFFSETS.iter()) {
            let corner_length = (off_x * off_x + off_y * off_y).sqrt();
            let alignment = if corner_length > 0. {
                (travel_dir.0 * off_x + travel_dir.1 * off_y) / corner_length
            } else {
                0.
            };
            // Corners pointing away from the direction of travel are the ones
            // that lag behind and stretch the cursor into a smear
            let lag = (-alignment).clamp(0., 1.);
            corner.speed = 1. - trail * lag;
            corner.start = corner.current;
            corner.t = 0.;
            slowest = slowest.min(corner.speed);
        }

        // Normalize so that the slowest corner, and therefore the animation as
        // a whole, takes exactly the configured duration regardless of which
        // way the cursor happened to move.
        if slowest > 0. {
            for corner in self.corners.iter_mut() {
                corner.speed /= slowest;
            }
        }

        self.animating = true;
    }

    fn snap(&mut self, destination: [(f32, f32); SMEAR_CORNER_COUNT], now: Instant) {
        for (corner, dest) in self.corners.iter_mut().zip(destination.iter()) {
            *corner = Corner::at(*dest);
        }
        self.animating = false;
        self.last_update = now;
    }

    fn positions(&self) -> [(f32, f32); SMEAR_CORNER_COUNT] {
        let mut positions = [(0., 0.); SMEAR_CORNER_COUNT];
        for (pos, corner) in positions.iter_mut().zip(self.corners.iter()) {
            *pos = corner.current;
        }
        positions
    }
}
