import React from "react";
import type { LucideIcon } from "lucide-react";

interface SettingsGroupProps {
  title?: string;
  /** A small icon before the title, to find the group at a glance. */
  icon?: LucideIcon;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  icon: Icon,
  description,
  children,
}) => {
  return (
    <div className="space-y-2">
      {title && (
        <div className="px-4">
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-ink-3 uppercase tracking-[.06em]">
            {Icon && <Icon aria-hidden="true" size={14} className="shrink-0" />}
            {title}
          </h2>
          {description && (
            <p className="text-xs text-ink-3 mt-1">{description}</p>
          )}
        </div>
      )}
      <div className="bg-raised border border-line rounded-[14px] overflow-visible">
        <div className="divide-y divide-line">{children}</div>
      </div>
    </div>
  );
};
