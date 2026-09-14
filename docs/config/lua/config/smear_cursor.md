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

It is **disabled by default**; set `enabled = true` to turn it on:

```lua
config.smear_cursor = {
  enabled = true,
}
```

## Options

* `enabled` - whether to draw the smear at all. Defaults to `false`.
* `duration_ms` - how long, in milliseconds, the whole animation takes. This is
  the main speed control: smaller values give a faster, shorter lived smear.
  Defaults to `130`.
* `trail_size` - how far the trailing corners lag behind the leading ones, in
  the range `0.0` to `1.0`. `0` moves the cursor rigidly with no stretching at
  all; values closer to `1` give a longer smear. Defaults to `0.7`.
* `easing` - the easing function applied to each corner as it travels. Accepts
  the same values as [visual_bell](visual_bell.md)'s easing functions. Defaults
  to `"EaseOut"`.
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
