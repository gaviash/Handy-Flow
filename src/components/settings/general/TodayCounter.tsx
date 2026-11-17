import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, events } from "@/bindings";
import { prefersReducedMotion } from "@/lib/utils/motion";

// Typing vs speaking speed, words per minute: the estimate behind "saved".
const TYPING_WPM = 40;
const SPEAKING_WPM = 150;

/** Rolls from the last shown value to `target`, fast then settling. */
function useRollingNumber(target: number, ms = 900): number {
  const [shown, setShown] = useState(0);
  const current = useRef(0);
  useEffect(() => {
    const from = current.current;
    if (from === target || prefersReducedMotion()) {
      current.current = target;
      setShown(target);
      return;
    }
    const start = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const p = Math.min((now - start) / ms, 1);
      const eased = p === 1 ? 1 : 1 - Math.pow(2, -10 * p);
      current.current = Math.round(from + (target - from) * eased);
      setShown(current.current);
      if (p < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [target, ms]);
  return shown;
}

/** Today's dictated words and the typing time they spared, live. */
export const TodayCounter: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [words, setWords] = useState<number | null>(null);
  // Bumped when a dictation lands while the page is open, to flash the figure.
  const [bump, setBump] = useState(0);
  const last = useRef<number | null>(null);

  useEffect(() => {
    let alive = true;
    const load = (live: boolean) =>
      commands.getDictationStats().then((r) => {
        if (!alive || r.status !== "ok") return;
        const next = r.data.words_today;
        if (live && last.current !== null && next > last.current)
          setBump((b) => b + 1);
        last.current = next;
        setWords(next);
      });
    load(false);
    const unlisten = events.historyUpdatePayload.listen((e) => {
      if (e.payload.action === "added" || e.payload.action === "updated")
        load(true);
    });
    // Back from the tray, possibly on a new day: count again.
    const unlistenShown = listen("main-window-shown", () => load(false));
    return () => {
      alive = false;
      unlisten.then((f) => f());
      unlistenShown.then((f) => f());
    };
  }, []);

  const minutes = Math.round(
    (words ?? 0) * (1 / TYPING_WPM - 1 / SPEAKING_WPM),
  );
  const rolledWords = useRollingNumber(words ?? 0);
  const rolledMinutes = useRollingNumber(minutes);
  if (words === null) return null;

  const number = new Intl.NumberFormat(i18n.language);
  const duration =
    rolledMinutes < 60
      ? new Intl.NumberFormat(i18n.language, {
          style: "unit",
          unit: "minute",
          unitDisplay: "short",
        }).format(rolledMinutes)
      : new Intl.NumberFormat(i18n.language, {
          style: "unit",
          unit: "hour",
          unitDisplay: "short",
          maximumFractionDigits: 1,
        }).format(rolledMinutes / 60);

  if (words === 0)
    return (
      <p className="text-sm text-ink-3 -mt-2">
        {t("settings.general.today.empty")}
      </p>
    );

  return (
    <div
      className="flex items-end gap-8 -mt-2 px-1"
      aria-live="polite"
      data-testid="today-counter"
    >
      <Figure
        value={number.format(rolledWords)}
        label={t("settings.general.today.words")}
        bump={bump}
      />
      <div className="w-px self-stretch bg-line my-1" aria-hidden />
      <Figure
        value={duration}
        label={t("settings.general.today.saved")}
        hint={t("settings.general.today.savedHint", {
          typing: TYPING_WPM,
          speaking: SPEAKING_WPM,
        })}
        bump={bump}
      />
    </div>
  );
};

const Figure: React.FC<{
  value: string;
  label: string;
  hint?: string;
  bump: number;
}> = ({ value, label, hint, bump }) => (
  <div title={hint}>
    <div
      // Remounting on each bump replays the accent glow.
      key={bump}
      className={`today-figure tabular-nums ${bump ? "animate-glow" : ""}`}
    >
      {value}
    </div>
    <div className="text-xs text-ink-2 mt-1">{label}</div>
  </div>
);
