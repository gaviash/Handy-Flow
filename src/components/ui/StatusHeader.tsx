import React from "react";
import { useTranslation } from "react-i18next";

export type StatusTone = "ok" | "warn" | "off";

const DOT: Record<StatusTone, string> = {
  ok: "bg-moss",
  warn: "bg-warning",
  off: "bg-ink-3",
};

/** The sentence a settings page opens on: what will happen, in words and not
 * only in colour. */
export const StatusHeader: React.FC<{
  tone: StatusTone;
  title: React.ReactNode;
  detail?: React.ReactNode;
  children?: React.ReactNode;
}> = ({ tone, title, detail, children }) => {
  const { t } = useTranslation();
  return (
    <div
      role="status"
      className={`rounded-[var(--radius-panel)] border p-4 text-sm ${
        tone === "warn"
          ? "border-warning bg-warning-wash"
          : "border-line bg-[color-mix(in_srgb,var(--color-logo-primary)_7%,var(--color-raised))]"
      }`}
    >
      <p className="flex flex-wrap items-center gap-x-2 font-medium text-text">
        <span className="inline-flex items-center gap-1.5 text-xs font-semibold text-ink-2">
          <span
            aria-hidden="true"
            className={`size-2 rounded-full ${DOT[tone]}`}
          />
          {t(`status.${tone}`)}
        </span>
        <span>{title}</span>
      </p>
      {detail && <p className="mt-1 text-ink-2">{detail}</p>}
      {children && <div className="mt-3">{children}</div>}
    </div>
  );
};
