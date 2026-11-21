# Horizon Fabrishot

An exact 1:1 Rust translation of [Fabrishot](https://github.com/ramidzkh/fabrishot)
by Ramid Khan (upstream Java, MIT (c) 2021 Ramid Khan). Carries the upstream
license and credit. Ported for the Horizon client by sparticle.

High-resolution, delay-timed screenshots with custom file names and
formats: the F9 screenshot key runs the original's CaptureTask state
machine (HUD hide, the frame-count delay, the resolution swap/restore),
the file naming carries the original's `%time%`/`%world%` substitution and
the suffix loop verbatim (including the `_3` second candidate), and the
writer encodes and saves real frames in the original's four formats
(PNG, JPG at quality 90, TGA, BMP).

## Build

```
cargo test --release   # 8 tests, green
```

Pinned to Horizon `f00125f61f794aaa3d4e950d301f353b35b6ee3d` and
horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, mod api 0.2.

## How the capture rides the mod API

The F9 key mapping starts the task; the render frame post hook drives
`onRenderTick`; the vanilla F2 screenshot hook carries the override path
(`override_screenshot_key` on: the vanilla read feeds the mod's file name,
format and `screenshot.success` message). The framebuffer resize and the
modded-resolution read have no mod API surface yet, so the F9 task's
capture frame completes with the file chosen and the F2 hook path is the
live capture (README seams below).

## Known seams (engine-stage; tracked)

- No framebuffer resize/read at the modded resolution (the 4K upscale
  waits for a surface).
- No world-name read (`%world%` rides `no_world`).
- No wall-clock read (the timestamp is UTC).
- The chat line's underline and click-to-open, and the narrator line.
- `custom_file_name` stays file-held in `config/fabrishot.properties`
  (no string setting kind).

Licence: MIT (LICENSE carries the upstream text).
