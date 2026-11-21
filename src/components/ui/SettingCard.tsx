import React from "react";
import { ChevronRight, type LucideIcon } from "lucide-react";

/** One topic of a settings page: its current value at a glance, opening its
 * sub-page. */
export const SettingCard = React.forwardRef<
  HTMLButtonElement,
  {
    icon: LucideIcon;
    title: string;
    summary: React.ReactNode;
    tone?: "warn";
    onOpen: () => void;
  }
>(({ icon: Icon, title, summary, tone, onOpen }, ref) => (
  <button
    ref={ref}
    type="button"
    onClick={onOpen}
    className="group flex min-h-16 w-full cursor-pointer items-center gap-3 rounded-[var(--radius-card)] border border-line bg-raised p-3 text-start transition-colors duration-150 hover:bg-wash focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
  >
    <span
      aria-hidden="true"
      className="flex size-10 shrink-0 items-center justify-center rounded-[10px] bg-wash text-ink-2 group-hover:text-text"
    >
      <Icon size={20} />
    </span>
    <span className="min-w-0 flex-1">
      <span className="block text-sm font-semibold text-text">{title}</span>
      <span
        className={`block truncate text-xs ${tone === "warn" ? "text-warning" : "text-ink-2"}`}
        title={typeof summary === "string" ? summary : undefined}
      >
        {summary}
      </span>
    </span>
    <ChevronRight
      aria-hidden="true"
      size={16}
      className="shrink-0 text-ink-3 rtl:rotate-180"
    />
  </button>
));
SettingCard.displayName = "SettingCard";

export const CardGrid: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">{children}</div>;
