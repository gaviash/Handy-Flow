import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { PALETTE_IDS, type PaletteId } from "@/lib/themes";
import { PalettePreview } from "./PalettePreview";

export function PalettePicker({
  value,
  onChange,
}: {
  value: PaletteId;
  onChange: (id: PaletteId) => void;
}) {
  const { t } = useTranslation();
  const group = useRef<HTMLDivElement>(null);
  return (
    <div
      ref={group}
      role="radiogroup"
      aria-label={t("settings.appearance.palette")}
      className="grid grid-cols-2 min-[900px]:grid-cols-4 gap-2.5"
    >
      {PALETTE_IDS.map((id, index) => (
        <button
          key={id}
          type="button"
          role="radio"
          aria-label={t(`settings.appearance.palettes.${id}.name`)}
          aria-checked={value === id}
          tabIndex={value === id ? 0 : -1}
          className={`rounded-[14px] border p-2 text-left transition-transform duration-150 ease-glide hover:-translate-y-0.5 ${value === id ? "border-accent-text outline outline-accent-text" : "border-line"}`}
          onClick={() => onChange(id)}
          onKeyDown={(event) => {
            const columns = window.matchMedia("(min-width: 900px)").matches
              ? 4
              : 2;
            const step =
              event.key === "ArrowRight"
                ? 1
                : event.key === "ArrowLeft"
                  ? -1
                  : event.key === "ArrowDown"
                    ? columns
                    : event.key === "ArrowUp"
                      ? -columns
                      : 0;
            if (!step) return;
            event.preventDefault();
            const next =
              (index + step + PALETTE_IDS.length) % PALETTE_IDS.length;
            group.current
              ?.querySelectorAll<HTMLButtonElement>("button")
              [next]?.focus();
          }}
        >
          <PalettePreview palette={id} />
          <span className="mt-2 flex items-center justify-between text-sm font-semibold">
            {t(`settings.appearance.palettes.${id}.name`)}
            {value === id && (
              <Check
                size={14}
                className="text-accent-text"
                aria-hidden="true"
              />
            )}
          </span>
          <span className="mt-1 block text-xs text-ink-2">
            {t(`settings.appearance.palettes.${id}.tagline`)}
          </span>
        </button>
      ))}
    </div>
  );
}
