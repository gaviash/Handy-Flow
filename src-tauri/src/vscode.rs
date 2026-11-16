//! Handy Adaptive: VS Code's "Screen Reader Optimized" mode
//! (`editor.accessibilitySupport: "on"`) makes its editor expose the text
//! around the cursor to UI Automation, which the context and the learning
//! read. Turned on in the user settings of each installed VS Code flavour.

const KEY: &str = "editor.accessibilitySupport";

/// Settings folders under %APPDATA% of VS Code and its forks.
const FLAVOURS: &[&str] = &["Code", "Code - Insiders", "Cursor", "VSCodium"];

/// What [`scan`] finds in a JSONC settings file.
struct Scan {
    /// Byte offset of the root object's `{`.
    open: usize,
    /// The root object has no entry.
    empty: bool,
    /// Byte range of the root-level value of [`KEY`], if set.
    value: Option<std::ops::Range<usize>>,
}

/// Skips whitespace and `//` / `/* */` comments from `i`; `None` on an
/// unclosed block comment.
fn skip(text: &str, mut i: usize) -> Option<usize> {
    let b = text.as_bytes();
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b[i..].starts_with(b"//") {
            i = b[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(b.len(), |p| i + p);
        } else if b[i..].starts_with(b"/*") {
            i += 2 + text[i + 2..].find("*/")? + 2;
        } else {
            return Some(i);
        }
    }
}

/// The end (closing quote + 1) of the string starting at `i`.
fn string_end(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return Some(j + 1),
            _ => j += 1,
        }
    }
    None
}

/// Reads the root object of a JSONC text: strings, comments and nesting are
/// skipped, so only a root-level [`KEY`] counts. `None` when the text isn't
/// one well-formed object.
fn scan(text: &str) -> Option<Scan> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    let (mut open, mut empty, mut value) = (None, false, None);
    let mut last_key: Option<std::ops::Range<usize>> = None;
    let mut wants_value = false;
    let mut closed = false;
    loop {
        i = skip(text, i)?;
        if i >= b.len() {
            break;
        }
        if closed {
            // Anything but comments after the root object: not ours to edit.
            return None;
        }
        match b[i] {
            b'"' => {
                let end = string_end(b, i)?;
                if wants_value {
                    value = Some(i..end);
                    wants_value = false;
                } else if depth == 1 {
                    last_key = Some(i..end);
                }
                i = end;
            }
            b':' if depth == 1 => {
                let key = last_key.take()?;
                wants_value = text[key].trim_matches('"') == KEY;
                i += 1;
            }
            b'{' | b'[' => {
                if depth == 0 {
                    if b[i] != b'{' {
                        return None;
                    }
                    open = Some(i);
                    empty = b.get(skip(text, i + 1)?) == Some(&b'}');
                }
                wants_value = false;
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                closed = depth == 0;
                i += 1;
            }
            _ => {
                if depth == 0 {
                    return None;
                }
                if wants_value {
                    // A bare value (true, 1, null…): up to the next delimiter.
                    let end = b[i..]
                        .iter()
                        .position(|&c| matches!(c, b',' | b'}' | b'/') || c.is_ascii_whitespace())
                        .map_or(b.len(), |p| i + p);
                    value = Some(i..end);
                    wants_value = false;
                    i = end;
                } else {
                    i += 1;
                }
            }
        }
    }
    closed.then_some(Scan {
        open: open?,
        empty,
        value,
    })
}

/// `text` with the setting on, or `None` when it already is (or the file
/// isn't one JSONC object). Comments and the rest of the file stay as they are.
pub fn with_screen_reader_on(text: &str) -> Option<String> {
    let s = scan(text)?;
    if let Some(v) = s.value {
        if &text[v.clone()] == "\"on\"" {
            return None;
        }
        let mut out = text.to_string();
        out.replace_range(v, "\"on\"");
        return Some(out);
    }
    // An empty object takes no comma after the new entry.
    let comma = if s.empty { "" } else { "," };
    Some(format!(
        "{}\n    \"{KEY}\": \"on\"{comma}{}",
        &text[..=s.open],
        &text[s.open + 1..]
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

/// Turns the mode on in every installed flavour; a file that can't be read,
/// parsed or written is left alone.
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
        // Only a missing file is created; one that can't be read stays as is.
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}\n".to_string(),
            Err(e) => {
                log::warn!("Couldn't read {flavour}'s settings ({e}); left as they are");
                continue;
            }
        };
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

    #[test]
    fn comments_and_nested_keys_are_not_the_setting() {
        // A brace in a comment before the object.
        let text = "// example {\n{\"editor.fontSize\": 14}";
        assert_eq!(
            with_screen_reader_on(text).unwrap(),
            "// example {\n{\n    \"editor.accessibilitySupport\": \"on\",\"editor.fontSize\": 14}"
        );
        // A commented-out or nested occurrence isn't the real one.
        let text = "{\n  // \"editor.accessibilitySupport\": \"off\",\n  \"[md]\": { \"editor.accessibilitySupport\": \"off\" },\n  \"editor.accessibilitySupport\": \"auto\" /* x */\n}";
        let out = with_screen_reader_on(text).unwrap();
        assert!(
            out.contains("// \"editor.accessibilitySupport\": \"off\""),
            "{out}"
        );
        assert!(
            out.contains("{ \"editor.accessibilitySupport\": \"off\" }"),
            "{out}"
        );
        assert!(
            out.contains("\"editor.accessibilitySupport\": \"on\" /* x */"),
            "{out}"
        );
        // A bare value, a string holding braces, a trailing comma.
        assert_eq!(
            with_screen_reader_on("{\"a\": \"}{\", \"editor.accessibilitySupport\": off,}")
                .unwrap(),
            "{\"a\": \"}{\", \"editor.accessibilitySupport\": \"on\",}"
        );
        // Not one object: untouched.
        assert_eq!(with_screen_reader_on("{} {}"), None);
        assert_eq!(with_screen_reader_on("{ /* open"), None);
        assert_eq!(with_screen_reader_on("[1]"), None);
    }
}
