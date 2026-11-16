import React from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import RecordingOverlay from "./RecordingOverlay";
import {
  applyAppearance,
  getStoredAppearance,
  syncAppearanceFromSettings,
  type Appearance,
} from "@/lib/utils/appearance";
import "@/i18n";

// A separate webview from the settings window, so the overlay has to paint its
// own document: last-known appearance before render (shared localStorage) to
// avoid a flash, reconcile with the persisted setting in case the overlay
// booted first, then follow live changes.
applyAppearance(getStoredAppearance());
syncAppearanceFromSettings();
listen<Appearance>("appearance-changed", (event) =>
  applyAppearance(event.payload),
);

// The overlay floats over other apps, so a stray right-click must not open the
// webview's default menu (Reload, Save as, Print, ...). Kept in dev for Inspect.
if (import.meta.env.PROD) {
  window.addEventListener("contextmenu", (e) => e.preventDefault());
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RecordingOverlay />
  </React.StrictMode>,
);
