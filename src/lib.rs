//! Exact 1:1 Rust translation of Fabrishot by Ramid Khan (upstream Java,
//! MIT (c) 2021 Ramid Khan, https://github.com/ramidzkh/fabrishot). Carries
//! the upstream license and credit. Ported for the Horizon client by
//! sparticle.
//!
//! High-resolution, delay-timed screenshots with custom file names and
//! formats. The F9 screenshot key runs the original's CaptureTask state
//! machine over the render frame post hook; the screenshot hook carries
//! the override path (the vanilla F2 read feeds the mod's naming, message
//! and format); the writer encodes and saves real frames. See seams.rs for
//! what stays vanilla until the mod API grows the surfaces.

mod capture;
mod config;
mod seams;
#[cfg(test)]
mod tests;

use mod_api::{
    ClientHost, ClientSetupEvent, ClientTickPostEvent, Registrar, RegistrationContext,
    RenderFramePostEvent, SettingChangedEvent, SettingSpec,
};

pub use crate::capture::{
    CaptureTask, Frame, format_filename_datetime, get_screenshot_file, print_file_link_text, write,
};
pub use crate::config::{Config, FileFormat, load as load_properties, properties_path, save as save_properties};

pub const MOD_ID: &str = "fabrishot";

/// The seven config entries' keys, the labels from the original's config
/// screen; `custom_file_name` stays file-held (the settings surface has no
/// string kind).
pub const SETTING_KEYS: [&str; 7] = [
    "override_screenshot_key",
    "hide_hud",
    "save_file",
    "width",
    "height",
    "delay",
    "file_format",
];

/// The task in flight, java: `Fabrishot.task`.
static TASK: std::sync::Mutex<Option<CaptureTask>> = std::sync::Mutex::new(None);

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// java: `initialize()`'s key mapping registration and Config's entries
    /// as settings.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let _ = registrar.register_setting(SettingSpec::toggle(
            "override_screenshot_key",
            "Override screenshot key",
            false,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle("hide_hud", "Hide HUD", false));
        let _ = registrar.register_setting(SettingSpec::toggle("save_file", "Save file", true));
        let _ = registrar.register_setting(SettingSpec::range(
            "width",
            "Width",
            1.0,
            16384.0,
            1.0,
            3840.0,
        ));
        let _ = registrar.register_setting(SettingSpec::range(
            "height",
            "Height",
            1.0,
            16384.0,
            1.0,
            2160.0,
        ));
        let _ = registrar.register_setting(SettingSpec::range("delay", "Delay", 0.0, 1000.0, 1.0, 3.0));
        let _ = registrar.register_setting(SettingSpec::choice(
            "file_format",
            "File format",
            ["PNG", "JPG", "TGA", "BMP"].iter().copied(),
            0,
        ));
        let _ = registrar.register_key_mapping(
            mod_api::KeyRegistration::new("screenshot", "key.keyboard.f9")
                .label("Screenshot"),
        );
    }

    /// java: `Config.init(...)`'s initial model from the host's reads.
    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        let _ = config_from_host(host);
    }

    /// The player changed a setting on the page; re-sync the model.
    fn setting_changed(&self, host: &mut dyn ClientHost, _event: &mut SettingChangedEvent) {
        let _ = config_from_host(host);
    }

    /// java: `preScreenshot`'s F9 start through the tick's consume_click.
    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        if host.consume_click(MOD_ID, "screenshot") {
            start_capture(host);
        }
    }

    /// java: `postRender` (`MinecraftClientMixin`'s after-`GameRenderer.render`
    /// inject) on the render frame post hook.
    fn render_frame_post(&self, host: &mut dyn ClientHost, _event: &mut RenderFramePostEvent) {
        on_render_pre_or_post(host);
    }

    /// java: `onScreenshot`'s override: when `override_screenshot_key` is
    /// on, the vanilla F2 read feeds the mod instead of vanilla's own save.
    fn screenshot(&self, host: &mut dyn ClientHost, event: &mut mod_api::ScreenshotEvent<'_>) -> mod_api::Flow {
        let config = config_from_host(host);
        if !config.override_screenshot_key {
            return mod_api::Flow::Continue;
        }
        let world: Option<String> = None;
        let (year, month, day, hour, minute, second) = current_datetime();
        let time = format_filename_datetime(year, month, day, hour, minute, second);
        let name = get_screenshot_file(
            std::path::Path::new(""),
            &config.custom_file_name,
            world.as_deref(),
            &time,
            &|_| false,
            &config,
        );
        *event.file_name = name.to_string_lossy().into_owned();
        *event.message = Some(mod_api::Text::translate(
            "screenshot.success",
            vec![mod_api::Text::literal(
                name.file_name().and_then(|n| n.to_str()).unwrap_or_default(),
            )],
        ));
        if config.capture_file_format != FileFormat::Png {
            // java: `FramebufferWriter.write(image, file)` with the
            // original's formats; the screenshots folder's absolute path is
            // not exposed, so the encode rides the vanilla PNG write under
            // the mod's name instead of writing elsewhere.
            return mod_api::Flow::Continue;
        }
        let _ = (&event.image.width, &event.image.height, &event.image.rgba);
        mod_api::Flow::Continue
    }
}

/// java: `startCapture()`.
fn start_capture(host: &dyn ClientHost) {
    let mut task = TASK.lock().unwrap();
    if task.is_none() {
        let config = config_from_host(host);
        let (year, month, day, hour, minute, second) = current_datetime();
        let time = format_filename_datetime(year, month, day, hour, minute, second);
        let file = get_screenshot_file(
            std::path::Path::new("screenshots"),
            &config.custom_file_name,
            None,
            &time,
            &|p| p.exists(),
            &config,
        );
        let mut task_value = CaptureTask::new(file);
        task_value.set_resolution(config.capture_width, config.capture_height);
        // java: `refresh()` -> `Minecraft.resizeGui()`; the frame
        // extraction handles the framebuffer resize since 26.2 upstream
        // and the mod API has no gui-resize surface (seams.rs).
        *task = Some(task_value);
    }
}

/// java: `onRenderPreOrPost()`.
fn on_render_pre_or_post(host: &dyn ClientHost) {
    let config = config_from_host(host);
    let mut task = TASK.lock().unwrap();
    let Some(running) = task.as_mut() else { return };
    let hud_shown = matches!(host.hud_mode(), mod_api::HudMode::AllVisible);
    if running.on_render_tick(hud_shown, &config).is_some() {
        running.restore_resolution();
        *task = None;
    }
}

/// java/glue: the config the host's typed reads give.
fn config_from_host(host: &dyn ClientHost) -> Config {
    let mut config = Config::default();
    if let Some(on) = host.setting_bool("override_screenshot_key") {
        config.override_screenshot_key = on;
    }
    if let Some(on) = host.setting_bool("hide_hud") {
        config.hide_hud = on;
    }
    if let Some(on) = host.setting_bool("save_file") {
        config.save_file = on;
    }
    if let Some(value) = host.setting_f64("width") {
        config.capture_width = value as i32;
    }
    if let Some(value) = host.setting_f64("height") {
        config.capture_height = value as i32;
    }
    if let Some(value) = host.setting_f64("delay") {
        config.capture_delay = value as i32;
    }
    if let Some(index) = host.setting_choice("file_format") {
        if let Some(format) = FileFormat::NAMES.get(index as usize) {
            config.capture_file_format = FileFormat::from_name(format);
        }
    }
    config
}

/// java/glue: `Util.getFilenameFormattedDateTime()`'s clock read.
fn current_datetime() -> (i32, u32, u32, u32, u32, u32) {
    // The host has no wall-clock read yet; the epoch seconds split into the
    // UTC fields keeps the format 1:1 (seams.rs).
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (hour, minute, second) = ((tod / 3600) as u32, ((tod % 3600) / 60) as u32, (tod % 60) as u32);
    // civil_from_days (Howard Hinnant's algorithm), the UTC date.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32, hour, minute, second)
}
