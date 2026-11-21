import React from "react";
import { ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";

/** Settings most people never need, folded until asked for. */
export const ExpertSection: React.FC<{
  summary: string;
  children: React.ReactNode;
}> = ({ summary, children }) => {
  const { t } = useTranslation();
  return (
    <details className="group rounded-[var(--radius-card)] border border-line">
      <summary className="flex cursor-pointer list-none items-center gap-2 rounded-[var(--radius-card)] px-4 py-3 text-sm text-ink-2 hover:text-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary [&::-webkit-details-marker]:hidden">
        <ChevronRight
          aria-hidden="true"
          size={16}
          className="transition-transform duration-200 ease-[var(--ease-glide)] group-open:rotate-90 motion-reduce:transition-none"
        />
        <span>
          <span className="font-medium text-text">{t("expert.title")}</span>
          {" — "}
          {summary}
        </span>
      </summary>
      <div className="space-y-2 px-2 pb-2">{children}</div>
    </details>
  );
};
