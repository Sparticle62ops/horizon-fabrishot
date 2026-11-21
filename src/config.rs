//! java: config/Config.java + config/FileFormat.java 1:1 (Ramid Khan's
//! java.util.Properties store with the same keys, defaults and
//! catch-anything-fails-then-save behaviour; the entries register as
//! settings with the original's labels, `custom_file_name` stays
//! file-held, the settings surface has no string kind).

use std::path::{Path, PathBuf};

/// java: config/FileFormat.java.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileFormat {
    Png,
    Jpg,
    Tga,
    Bmp,
}

impl FileFormat {
    /// java: `"." + name().toLowerCase(Locale.ROOT)`.
    pub fn extension(self) -> String {
        match self {
            FileFormat::Png => ".png",
            FileFormat::Jpg => ".jpg",
            FileFormat::Tga => ".tga",
            FileFormat::Bmp => ".bmp",
        }
        .to_string()
    }

    /// java: `FileFormat.valueOf(properties.getProperty("file_format"))`,
    /// the enum's name.
    pub fn from_name(name: &str) -> FileFormat {
        match name {
            "JPG" => FileFormat::Jpg,
            "TGA" => FileFormat::Tga,
            "BMP" => FileFormat::Bmp,
            _ => FileFormat::Png,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FileFormat::Png => "PNG",
            FileFormat::Jpg => "JPG",
            FileFormat::Tga => "TGA",
            FileFormat::Bmp => "BMP",
        }
    }

    pub const NAMES: [&'static str; 4] = ["PNG", "JPG", "TGA", "BMP"];
}

/// java: Config, same defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub custom_file_name: String,
    pub override_screenshot_key: bool,
    pub hide_hud: bool,
    pub save_file: bool,
    pub capture_width: i32,
    pub capture_height: i32,
    pub capture_delay: i32,
    pub capture_file_format: FileFormat,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            custom_file_name: "huge_%time%".to_string(),
            override_screenshot_key: false,
            hide_hud: false,
            save_file: true,
            capture_width: 3840,
            capture_height: 2160,
            capture_delay: 3,
            capture_file_format: FileFormat::Png,
        }
    }
}

/// java: the `static` block's java.util.Properties load from
/// `config/fabrishot.properties`: any failure resets the whole config to
/// its defaults (`catch (Exception ignored) { save(); }`).
pub fn load(properties: &str) -> Config {
    let mut config = Config::default();
    let mut seen: Vec<(String, String)> = Vec::new();
    for line in properties.lines() {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let value = line[eq + 1..].trim().to_string();
            seen.push((key, value));
        }
    }
    let get = |key: &str| seen.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    // java: Integer.parseInt / Boolean.parseBoolean / valueOf throw on a
    // missing or malformed key, and the catch resets to defaults.
    let Some(custom_file_name) = get("custom_file_name") else {
        return Config::default();
    };
    let Some(override_screenshot_key) = get("override_screenshot_key").map(parse_boolean) else {
        return Config::default();
    };
    let Some(hide_hud) = get("hide_hud").map(parse_boolean) else {
        return Config::default();
    };
    let Some(save_file) = get("save_file").map(parse_boolean) else {
        return Config::default();
    };
    let Some(capture_width) = get("width").and_then(|v| v.parse::<i32>().ok()) else {
        return Config::default();
    };
    let Some(capture_height) = get("height").and_then(|v| v.parse::<i32>().ok()) else {
        return Config::default();
    };
    let Some(capture_delay) = get("delay").and_then(|v| v.parse::<i32>().ok()) else {
        return Config::default();
    };
    let Some(file_format) = get("file_format").map(|v| FileFormat::from_name(&v)) else {
        return Config::default();
    };
    config.custom_file_name = custom_file_name;
    config.override_screenshot_key = override_screenshot_key;
    config.hide_hud = hide_hud;
    config.save_file = save_file;
    config.capture_width = capture_width;
    config.capture_height = capture_height;
    config.capture_delay = capture_delay;
    config.capture_file_format = file_format;
    config
}

/// java: `Boolean.parseBoolean` (case-insensitive "true", everything else false).
fn parse_boolean(value: String) -> bool {
    value.eq_ignore_ascii_case("true")
}

/// java: `Config.save()`, the properties writer.
pub fn save(config: &Config) -> String {
    format!(
        "#fabrishot configuration\n#Sat Jan 01 00:00:00 UTC 2026\ncustom_file_name={}\noverride_screenshot_key={}\nhide_hud={}\nsave_file={}\nwidth={}\nheight={}\ndelay={}\nfile_format={}\n",
        config.custom_file_name,
        config.override_screenshot_key,
        config.hide_hud,
        config.save_file,
        config.capture_width,
        config.capture_height,
        config.capture_delay,
        config.capture_file_format.name(),
    )
}

/// java: `FabricLoader.getInstance().getConfigDir().resolve("fabrishot.properties")`.
pub fn properties_path(config_dir: &Path) -> PathBuf {
    config_dir.join("fabrishot.properties")
}
