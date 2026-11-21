//! Tests against the upstream ground truth (Ramid Khan's Java, read 1:1).

use super::{Frame, write};
use crate::config::{Config, FileFormat, load as load_properties, save as save_properties};
use crate::capture::{
    CaptureTask, format_filename_datetime, get_screenshot_file, print_file_link_text, write, Frame,
};
use std::path::{Path, PathBuf};

#[test]
fn default_config_matches_upstream() {
    let c = Config::default();
    assert_eq!(c.custom_file_name, "huge_%time%");
    assert!(!c.override_screenshot_key);
    assert!(!c.hide_hud);
    assert!(c.save_file);
    assert_eq!(c.capture_width, 3840);
    assert_eq!(c.capture_height, 2160);
    assert_eq!(c.capture_delay, 3);
    assert_eq!(c.capture_file_format, FileFormat::Png);
}

#[test]
fn properties_round_trip_and_catch_all_reset() {
    let c = Config::default();
    let text = save_properties(&c);
    assert!(text.contains("custom_file_name=huge_%time%"));
    assert!(text.contains("width=3840"));
    assert!(text.contains("file_format=PNG"));
    assert_eq!(load_properties(&text), c);
    // java: any parse failure resets to defaults.
    assert_eq!(load_properties("width=3840\n"), Config::default());
    assert_eq!(load_properties("width=abc\n"), Config::default());
    let mut c = Config::default();
    c.hide_hud = true;
    c.capture_file_format = FileFormat::Jpg;
    let text = save_properties(&c);
    assert!(text.contains("hide_hud=true"));
    assert_eq!(load_properties(&text), c);
}

#[test]
fn extensions_match_the_original() {
    assert_eq!(FileFormat::Png.extension(), ".png");
    assert_eq!(FileFormat::Jpg.extension(), ".jpg");
    assert_eq!(FileFormat::Tga.extension(), ".tga");
    assert_eq!(FileFormat::Bmp.extension(), ".bmp");
}

#[test]
fn task_frames_the_delay_and_reports_the_capture_frame() {
    let mut c = Config::default();
    c.capture_delay = 3;
    c.hide_hud = true;
    let mut task = CaptureTask::new(PathBuf::from("screenshots/x.png"));
    task.set_resolution(3840, 2160);
    assert_eq!(task.resolution(), (3840, 2160));
    // frame 0: the HUD toggles (shown -> hidden), no capture.
    assert!(task.on_render_tick(true, &c).is_none());
    // frame 1 and 2: inside the delay, no capture.
    assert!(task.on_render_tick(false, &c).is_none());
    assert!(task.on_render_tick(false, &c).is_none());
    // frame 3: the capture frame names the file.
    assert_eq!(task.on_render_tick(false, &c), Some(Path::new("screenshots/x.png")));
    task.restore_resolution();
    assert_eq!(task.resolution(), (0, 0));
}

#[test]
fn filename_datetime_is_the_vanilla_format() {
    assert_eq!(format_filename_datetime(2026, 11, 21, 9, 5, 3), "2026-11-21_09.05.03");
}

#[test]
fn filename_prefix_substitution_and_the_suffix_loop() {
    let c = Config::default();
    let existing = |p: &Path| p.as_os_str().to_string_lossy().ends_with("huge_2026-11-21_09.05.03.png");
    let file = get_screenshot_file(
        Path::new("screenshots"),
        "huge_%time%",
        None,
        "2026-11-21_09.05.03",
        &existing,
        &c,
    );
    assert_eq!(
        file.to_str().unwrap(),
        "huge_2026-11-21_09.05.03_3.png",
        "java: the second candidate is `prefix_3`, the post-increment quirk"
    );
    let none_existing = |_: &Path| false;
    let file = get_screenshot_file(
        Path::new("screenshots"),
        "huge_%time%_%world%",
        Some("My World"),
        "2026-11-21_09.05.03",
        &none_existing,
        &c,
    );
    assert_eq!(file.to_str().unwrap(), "huge_2026-11-21_09.05.03_My World.png");
    // java: the null world rides `no_world`.
    let file = get_screenshot_file(
        Path::new("screenshots"),
        "%world%",
        None,
        "t",
        &none_existing,
        &c,
    );
    assert_eq!(file.to_str().unwrap(), "no_world.png");
}

#[test]
fn message_line_names_the_file() {
    assert_eq!(print_file_link_text(Path::new("screenshots/huge_1.png")), "Saved screenshot as huge_1.png");
}

#[test]
fn writer_encodes_and_saves_real_frames_in_all_formats() {
    // A 2x2 RGBA frame: red, green, blue, white.
    let frame = Frame {
        width: 2,
        height: 2,
        rgba: vec![
            255, 0, 0, 255, 0, 255, 0, 255,
            0, 0, 255, 255, 255, 255, 255, 255,
        ],
    };
    for format in [FileFormat::Png, FileFormat::Jpg, FileFormat::Tga, FileFormat::Bmp] {
        let mut c = Config::default();
        c.capture_file_format = format;
        let dir = std::env::temp_dir().join(format!("fabrishot-test-{format:?}"));
        let file = dir.join("shot").with_extension(format.extension().trim_start_matches('.'));
        write(&frame, &file, &c).expect("the encode and save");
        let bytes = std::fs::read(&file).expect("the file");
        assert!(!bytes.is_empty(), "{format:?} wrote a file");
        // TGA has no magic bytes, so the reader can't sniff it: load with
        // the extension's format named.
        let decoded = image::ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .expect("the reader")
            .decode()
            .unwrap_or_else(|_| {
                let fmt = match format {
                    FileFormat::Png => image::ImageFormat::Png,
                    FileFormat::Jpg => image::ImageFormat::Jpeg,
                    FileFormat::Tga => image::ImageFormat::Tga,
                    FileFormat::Bmp => image::ImageFormat::Bmp,
                };
                image::ImageReader::with_format(std::io::Cursor::new(&bytes), fmt)
                    .decode()
                    .expect("the encoded file decodes")
            });
        assert_eq!((decoded.width(), decoded.height()), (2, 2), "{format:?}");
        assert_eq!(format.extension(), format!(".{}", file.extension().unwrap().to_string_lossy()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
