---
tags:
  - appearance
  - cursor
---
# `smear_cursor`

!!! note
    This option is **not** part of upstream wezterm; it only exists in this
    fork. See [Sharing a config with a stock wezterm](#sharing-a-config-with-a-stock-wezterm)
    below before you put it in a `wezterm.lua` that both builds read.

A neovide style "smear" (or trail) animation for the cursor. When the cursor
moves, the quad that makes up the cursor is stretched back towards where it came
from, and the trailing corners then catch up, so the smear closes up again.

Each corner chases the cursor exponentially, covering a fraction of its
remaining distance per second, rather than running a fixed length animation
from wherever it started. That is what keeps it smooth when the cursor does not
stand still - a held down key, or a command writing out a screenful of text -
because the corners settle at a constant distance behind the cursor instead of
restarting, and falling further behind, on every step.

It is **disabled by default**; set `enabled = true` to turn it on:

```lua
config.smear_cursor = {
  enabled = true,
}
```

## Options

* `enabled` - whether to draw the smear at all. Defaults to `false`.
* `duration_ms` - how long, in milliseconds, a short movement takes to close
  the smear back up. This is the main speed control: smaller values give a
  faster, shorter lived smear. Defaults to `130`.
* `max_duration_ms` - how long, in milliseconds, a movement spanning 25 cells or
  more is allowed to take. The duration scales with the distance between this
  and `duration_ms`, so that a long jump does not cross most of the window in
  the first frame or two the way it would if every movement took `duration_ms`.
  Defaults to `250`.
* `trail_size` - how far the trailing corners lag behind the leading ones, in
  the range `0.0` to `1.0`. `0` moves the cursor rigidly with no stretching at
  all; values closer to `1` give a longer smear. Defaults to `0.7`.
* `opacity` - alpha applied to the smear color, in the range `0.0` to `1.0`.
  Defaults to `1.0`.
* `color` - the color of the smear. Defaults to the cursor color from your
  color scheme.
* `min_distance_cells` - cursor movements shorter than this many cells snap
  instead of animating, so that ordinary typing doesn't smear. Defaults to
  `0.5`.
* `max_distance_cells` - cursor movements longer than this many cells snap
  instead of animating, which is useful to avoid a smear across the whole
  window when the cursor jumps. `0` disables the limit, which is the default.
* `above_text` - when `true` the smear is drawn over the text, the way neovide
  does it. When `false`, the default, it is drawn behind the text so that the
  characters it passes over stay readable.
* `smear_scroll` - in a full screen application, smear the cursor when the view
  scrolls under it. Defaults to `true`. Commands like vim's `ctrl-d` move the
  cursor half a page through the buffer but leave it on the same screen row, so
  the cursor rect does not change and there is nothing for a smear anchored to
  the screen to chase. With this on the smear is anchored to the text instead,
  so it shows the cursor travelling through the buffer. A scroll can only drag
  the smear in from one screenful away, so jumping to the end of a long file
  does not smear in from thousands of rows off screen. This never applies to the
  main screen, where `smear_output` governs scrolling output instead.
* `smear_output` - when `true`, the smear follows the exact cursor position
  while the pane is printing. Defaults to `false`: a command printing a
  screenful of text walks the cursor from the end of one line to the start of
  the next once per frame, and following that horizontal bounce reads as a
  zigzag rather than as a trail. With the default the smear still follows the
  output down the screen, so it stays with the text as it scrolls by, but holds
  its horizontal position until the output stops and then settles across to the
  real column. Typing and cursor keys smear either way, as does everything a
  full screen application such as vim or less does, since those drive the
  cursor deliberately. Scrolling the viewport out from under the cursor never
  smears.

A longer, more obvious smear:

```lua
config.smear_cursor = {
  enabled = true,
  duration_ms = 220,
  trail_size = 0.85,
  opacity = 0.9,
}
```

A quick, subtle one:

```lua
config.smear_cursor = {
  enabled = true,
  duration_ms = 70,
  trail_size = 0.5,
}
```

The animation is driven at up to [max_fps](max_fps.md) frames per second while
it is running, and costs nothing when the cursor is at rest.

## Sharing a config with a stock wezterm

An upstream wezterm rejects config keys it doesn't know about, so a bare
`config.smear_cursor = { ... }` would break your config for any stock build that
reads the same file. This fork sets `wezterm.has_smear_cursor`, which is simply
`nil` on a stock build, so guard the assignment with it:

```lua
local wezterm = require 'wezterm'

if wezterm.has_smear_cursor then
  config.smear_cursor = {
    enabled = true,
  }
end
```
