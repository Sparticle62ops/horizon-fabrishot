//! Tracked seams, engine-stage: the pinned mod API (f00125f / e59da909,
//! api 0.2) has no counterpart for these yet.
//!
//! - No framebuffer resize/read at the modded resolution: upstream swaps
//!   the window size to `CAPTURE_WIDTH`x`CAPTURE_HEIGHT`, waits
//!   `CAPTURE_DELAY` frames and reads the main render target back
//!   (`Screenshot.takeScreenshot`). The mod API has no window/framebuffer
//!   resize and no framebuffer read outside the vanilla F2 screenshot
//!   hook, so the F9 task's capture frame completes with the file chosen
//!   and nothing written; the live read/encode/save path is the F2 hook
//!   with `override_screenshot_key` on.
//! - No world-name read: `getScreenshotFile`'s `%world%` substitution has
//!   no client wire (the singleplayer level name or the server's name), so
//!   the glue passes `None` and the name rides `no_world` upstream's own
//!   null branch.
//! - No wall-clock read: `Util.getFilenameFormattedDateTime()` is
//!   reproduced from the process clock in UTC; the vanilla line uses the
//!   system's local time.
//! - The chat line's underline and `ClickEvent.OpenFile` have no
//!   counterpart in the settings hook's message field, which carries plain
//!   text; the translated `screenshot.success` line and the file name ride
//!   as text. The narrator line has no counterpart.
//! - The config screen bridges (`ModMenuBridge`, `ClothConfigBridge`,
//!   `InstallClothConfigScreen`, `ScalingPresetEntry`) map onto the
//!   settings page: the seven registerable entries register as settings
//!   (toggles, sliders with the port's 1..=16384 bounds and the delay's
//!   0..=1000, the format choice); `custom_file_name` stays file-held (no
//!   string setting kind), read from and saved to
//!   `config/fabrishot.properties` in the original's java.util.Properties
//!   form.
//! - `RenderscaleCommonClassMixin` (the RenderScale integration) has no
//!   counterpart; the resize callback
//!   (`FramebufferCaptureCallback.EVENT`) rides the writer's call site.
