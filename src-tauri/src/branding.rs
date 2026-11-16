//! The app's public name. Everything a user reads as the app's name comes from
//! here or from `tauri.conf.json`; the data paths (`identifier`, the portable
//! marker) deliberately keep the original Handy values so existing settings,
//! history and models are found after the rename.

pub const APP_NAME: &str = "Handy Flow";
/// Title of the recording overlay window (never shown in the taskbar).
pub const OVERLAY_TITLE: &str = "Handy Flow Overlay";

#[cfg(test)]
mod tests {
    const CONF: &str = include_str!("../tauri.conf.json");

    fn conf() -> serde_json::Value {
        serde_json::from_str(CONF).expect("tauri.conf.json is valid JSON")
    }

    #[test]
    fn product_name_is_handy_flow() {
        let v = conf();
        assert_eq!(v["productName"], "Handy Flow");
        assert_eq!(v["mainBinaryName"], "Handy Flow");
    }

    #[test]
    fn identifier_never_changes() {
        assert_eq!(conf()["identifier"], "com.pais.handy");
    }

    #[test]
    fn upstream_updater_is_unreachable() {
        let v = conf();
        assert_eq!(v["plugins"]["updater"]["endpoints"], serde_json::json!([]));
        assert!(!CONF.contains("cjpais/Handy/releases"));
    }

    #[test]
    fn updater_is_forced_off() {
        assert!(crate::settings::update_checks_forced_disabled());
    }

    #[test]
    fn tray_tooltip_names_handy_flow() {
        assert!(crate::tray::tray_tooltip().starts_with("Handy Flow v"));
    }
}
