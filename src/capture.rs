//! java: capture/CaptureTask.java + capture/FramebufferWriter.java + the
//! event callbacks and the core of Fabrishot.java 1:1 (Ramid Khan, MIT).
//!
//! The capture's read at the modded resolution rides the seam (no
//! framebuffer resize/read on the mod API yet); everything the task does
//! around it is ported whole, and the writer encodes real frames on the
//! screenshot hook's live path.

use std::path::{Path, PathBuf};

use crate::config::Config;

/// java: the CaptureTask state machine.
#[derive(Debug)]
pub struct CaptureTask {
    file: PathBuf,
    hud_hidden: bool,
    frame: i32,
    prev_width: i32,
    prev_height: i32,
    /// java: the `Window` swap, modeled; the actual resize is a seam.
    width: i32,
    height: i32,
}

impl CaptureTask {
    /// java: `new CaptureTask(minecraft, file)`.
    pub fn new(file: PathBuf) -> CaptureTask {
        CaptureTask {
            file,
            hud_hidden: false,
            frame: 0,
            prev_width: 0,
            prev_height: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// java: `setResolution`: remember the current window size, apply the
    /// modded one.
    pub fn set_resolution(&mut self, width: i32, height: i32) {
        self.prev_width = self.width;
        self.prev_height = self.height;
        self.width = width;
        self.height = height;
    }

    /// java: `restoreResolution`.
    pub fn restore_resolution(&mut self) {
        self.width = self.prev_width;
        self.height = self.prev_height;
    }

    pub fn resolution(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// java: `onRenderTick`, the frame counter and the HUD toggle verbatim.
    /// `hud_shown` is the HUD's current shown state
    /// (`!minecraft.gui.hud.isHidden()`), `hud_shown_at_frame0` the value
    /// the caller read on frame 0 for the restore. Returns `Some(file)` on
    /// the capture frame (java: the `Screenshot.takeScreenshot` step, whose
    /// framebuffer read is the seam) and `None` while the task runs.
    pub fn on_render_tick(
        &mut self,
        hud_shown: bool,
        config: &Config,
    ) -> Option<&Path> {
        if self.frame == 0 {
            self.hud_hidden = !hud_shown;
            if hud_shown && config.hide_hud {
                // java: `minecraft.gui.hud.toggle()`; the host call is the
                // glue's set_hud_mode.
                self.hud_hidden = true;
            }
            self.frame += 1;
            None
        } else if self.frame < config.capture_delay {
            self.frame += 1;
            None
        } else {
            // java: `Screenshot.takeScreenshot(...)` -> FramebufferWriter
            // when SAVE_FILE; the read rides the seam, the write path is
            // [`FramebufferWriter::write`].
            if !self.hud_hidden && config.hide_hud {
                // java: restore the HUD after the capture.
                self.hud_hidden = false;
            }
            Some(self.file.as_path())
        }
    }
}

/// java: `Util.getFilenameFormattedDateTime()`, vanilla's
/// `yyyy-MM-dd_HH.mm.ss` file timestamp.
pub fn format_filename_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> String {
    format!("{year:04}-{month:02}-{day:02}_{hour:02}.{minute:02}.{second:02}")
}

/// java: `getScreenshotFile(Minecraft)`, the prefix substitution and the
/// suffix loop verbatim (including the post-increment quirk: the second
/// candidate is `<prefix>_3`, java's `i++ == 1` evaluates the condition on
/// the old value and the `_ + i` branch on the new one).
pub fn get_screenshot_file(
    dir: &Path,
    prefix_in: &str,
    world: Option<&str>,
    formatted_time: &str,
    exists: &dyn Fn(&Path) -> bool,
    config: &Config,
) -> PathBuf {
    let _ = dir;
    let world = world.unwrap_or("no_world");
    let prefix = prefix_in
        .replace("%time%", formatted_time)
        .replace("%world%", world);

    let mut i = 1;
    loop {
        let file = prefix.clone() + (if i == 1 { "".to_string() } else { format!("_{}", i + 1) }).as_str() + config.capture_file_format.extension().as_str();
        let path = PathBuf::from(file);
        if !exists(&path) {
            return path;
        }
        i += 1;
    }
}

/// java: `printFileLink`'s chat line: the translatable
/// `screenshot.success` ("Saved screenshot as %s") with the file's name,
/// underlined and click-to-open upstream. Returns the message text; the
/// underline and the `OpenFile` click event ride the seam (the settings
/// hook's message field carries plain text).
pub fn print_file_link_text(file: &Path) -> String {
    format!("Saved screenshot as {}", file.file_name().and_then(|n| n.to_str()).unwrap_or_default())
}

/// java: capture/FramebufferWriter.java, the STBImageWrite switch over the
/// `image` crate's encoders (the Rust STB equivalent): PNG, JPG at the
/// original's 90 quality, TGA and BMP, from the frame's RGBA8.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn write(image: &Frame, file: &Path, config: &Config) -> Result<(), String> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    use image::ImageEncoder;
    let color = image::ExtendedColorType::Rgba8;
    let file_out = std::fs::File::create(file).map_err(|e| e.to_string())?;
    let mut buffered = std::io::BufWriter::new(file_out);
    match config.capture_file_format {
        crate::config::FileFormat::Png => image::codecs::png::PngEncoder::new(buffered)
            .write_image(&image.rgba, image.width, image.height, color)
            .map_err(|e| e.to_string()),
        crate::config::FileFormat::Jpg => {
            // java: `image.format().components()` is 4 (RGBA) into STB's
            // writer, which packs 4-channel JPEG; the Rust crate's JPEG
            // encoder takes 3 channels, so the alpha is dropped at the
            // encoder (RGB8, no alpha).
            let mut rgb = Vec::with_capacity(image.rgba.len() / 4 * 3);
            for px in image.rgba.chunks_exact(4) {
                rgb.extend_from_slice(&px[..3]);
            }
            image::codecs::jpeg::JpegEncoder::new_with_quality(buffered, 90)
                .write_image(&rgb, image.width, image.height, image::ExtendedColorType::Rgb8)
                .map_err(|e| e.to_string())
        }
        crate::config::FileFormat::Tga => image::codecs::tga::TgaEncoder::new(buffered)
            .write_image(&image.rgba, image.width, image.height, color)
            .map_err(|e| e.to_string()),
        crate::config::FileFormat::Bmp => image::codecs::bmp::BmpEncoder::new(&mut buffered)
            .write_image(&image.rgba, image.width, image.height, color)
            .map_err(|e| e.to_string()),
    }
}
