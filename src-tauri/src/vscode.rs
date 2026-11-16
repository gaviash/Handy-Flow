//! Handy Adaptive: VS Code's "Screen Reader Optimized" mode
//! (`editor.accessibilitySupport: "on"`) makes its editor expose the text
//! around the cursor to UI Automation, which the context and the learning
//! read. Turned on in the user settings of each installed VS Code flavour.

use once_cell::sync::Lazy;
use regex::Regex;

const KEY: &str = "editor.accessibilitySupport";

/// Settings folders under %APPDATA% of VS Code and its forks.
const FLAVOURS: &[&str] = &["Code", "Code - Insiders", "Cursor", "VSCodium"];

/// `"editor.accessibilitySupport": <any value>`.
static SETTING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""editor\.accessibilitySupport"\s*:\s*("[^"]*"|[A-Za-z]+)"#).expect("valid regex")
});

/// `text` with the setting on, or `None` when it already is (or the file isn't
/// a JSON object). Comments and the rest of the file are kept as they are.
pub fn with_screen_reader_on(text: &str) -> Option<String> {
    if let Some(m) = SETTING.captures(text) {
        let value = m.get(1)?;
        if value.as_str() == "\"on\"" {
            return None;
        }
        let mut out = text.to_string();
        out.replace_range(value.range(), "\"on\"");
        return Some(out);
    }
    let open = text.find('{')?;
    let rest = &text[open + 1..];
    // An empty object takes no comma after the new entry.
    let comma = if rest.trim_start().starts_with('}') {
        ""
    } else {
        ","
    };
    Some(format!(
        "{}\n    \"{KEY}\": \"on\"{comma}{}",
        &text[..=open],
        rest
    ))
}

/// Settings → Context: turns the mode on now when switched on.
#[tauri::command]
#[specta::specta]
pub fn change_vscode_screen_reader_setting(app: tauri::AppHandle, enabled: bool) {
    let mut settings = crate::settings::get_settings(&app);
    settings.vscode_screen_reader = enabled;
    crate::settings::write_settings(&app, settings);
    if enabled {
        std::thread::spawn(ensure_screen_reader_mode);
    }
}

/// Turns the mode on in every installed flavour; a file that can't be read
/// or written is left alone.
pub fn ensure_screen_reader_mode() {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return;
    };
    for flavour in FLAVOURS {
        let path = std::path::Path::new(&appdata)
            .join(flavour)
            .join("User")
            .join("settings.json");
        // A flavour that was never started has no settings folder: skip it.
        if !path.parent().is_some_and(|p| p.is_dir()) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}\n".to_string());
        let Some(updated) = with_screen_reader_on(&text) else {
            continue;
        };
        // Written aside then renamed, so VS Code never reads half a file.
        let tmp = path.with_extension("json.handy-tmp");
        match std::fs::write(&tmp, updated).and_then(|()| std::fs::rename(&tmp, &path)) {
            Ok(()) => log::info!("Turned on Screen Reader Optimized mode in {flavour}"),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                log::warn!("Couldn't turn on Screen Reader Optimized mode in {flavour}: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_is_added_or_switched_on_and_comments_stay() {
        let text = "// mine\n{\n    \"editor.fontSize\": 14, // big\n}\n";
        let out = with_screen_reader_on(text).unwrap();
        assert_eq!(
            out,
            "// mine\n{\n    \"editor.accessibilitySupport\": \"on\",\n    \"editor.fontSize\": 14, // big\n}\n"
        );
        assert_eq!(with_screen_reader_on(&out), None, "already on");
        let auto = "{ \"editor.accessibilitySupport\" : \"auto\", \"a\": 1 }";
        assert_eq!(
            with_screen_reader_on(auto).unwrap(),
            "{ \"editor.accessibilitySupport\" : \"on\", \"a\": 1 }"
        );
        assert_eq!(
            with_screen_reader_on("{}").unwrap(),
            "{\n    \"editor.accessibilitySupport\": \"on\"}"
        );
        assert_eq!(with_screen_reader_on("not json"), None);
    }
}
