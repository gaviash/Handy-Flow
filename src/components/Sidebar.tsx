import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Brush,
  Cpu,
  FlaskConical,
  History,
  House,
  Info,
  SlidersHorizontal,
  Sparkles,
  BookOpen,
  Eye,
} from "lucide-react";
import HandyFlowWordmark from "./icons/HandyFlowWordmark";
import { useSettings } from "../hooks/useSettings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  AboutSettings,
  PostProcessingSettings,
  ModelsSettings,
  AppearanceSettings,
  VocabularySettings,
  ContextPage,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  strokeWidth?: number | string;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  /** `null` when App renders the section itself (Debug, with its handlers). */
  component: React.ComponentType | null;
  enabled: (settings: any) => boolean;
  /** Draw a divider above this entry. */
  dividerBefore?: boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "sidebar.general",
    icon: House,
    component: GeneralSettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    // Always shown: the page itself offers to turn the rewrite back on.
    enabled: () => true,
  },
  vocabulary: {
    labelKey: "sidebar.vocabulary",
    icon: BookOpen,
    component: VocabularySettings,
    enabled: () => true,
  },
  context: {
    labelKey: "sidebar.context",
    icon: Eye,
    component: ContextPage,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  appearance: {
    labelKey: "sidebar.appearance",
    icon: Brush,
    component: AppearanceSettings,
    enabled: () => true,
    dividerBefore: true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: SlidersHorizontal,
    component: AdvancedSettings,
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: null,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  // One shared highlight glides to the active item instead of each button
  // painting its own; `moved` keeps the first placement from sliding in.
  const buttons = useRef(new Map<SidebarSection, HTMLButtonElement>());
  const [pill, setPill] = useState<{ top: number; height: number } | null>(
    null,
  );
  const [moved, setMoved] = useState(false);
  const sectionIds = availableSections.map((s) => s.id).join();
  useLayoutEffect(() => {
    const el = buttons.current.get(activeSection);
    if (!el) return setPill(null);
    setPill({ top: el.offsetTop, height: el.offsetHeight });
  }, [activeSection, sectionIds]);
  useEffect(() => {
    if (pill && !moved) requestAnimationFrame(() => setMoved(true));
  }, [pill, moved]);

  return (
    <aside
      className="flex w-[212px] shrink-0 flex-col gap-[18px] bg-side px-3 pt-[18px] pb-3 text-side-ink border-e border-[color-mix(in_oklab,var(--side-ink)_8%,transparent)]"
      style={
        {
          "--wordmark-flow": "var(--wordmark-side, var(--accent-text))",
        } as React.CSSProperties
      }
    >
      <HandyFlowWordmark size={26} className="px-[10px] pt-[2px]" />
      <nav
        aria-label={t("sidebar.label")}
        className="relative flex flex-col gap-[2px]"
      >
        {pill && (
          <div
            aria-hidden
            data-testid="sidebar-pill"
            className={`absolute inset-x-0 top-0 rounded-[10px] bg-side-active ${
              moved
                ? "transition-[translate,height] duration-300 ease-glide"
                : ""
            }`}
            style={{ translate: `0 ${pill.top}px`, height: pill.height }}
          />
        )}
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;
          return (
            <React.Fragment key={section.id}>
              {"dividerBefore" in section && section.dividerBefore && (
                <div
                  aria-hidden
                  className="mx-[10px] my-2 h-px bg-[color-mix(in_oklab,var(--side-ink)_10%,transparent)]"
                />
              )}
              <button
                type="button"
                ref={(el) => {
                  if (el) buttons.current.set(section.id, el);
                  else buttons.current.delete(section.id);
                }}
                aria-current={isActive ? "page" : undefined}
                onClick={() => onSectionChange(section.id)}
                className={`relative flex w-full items-center gap-[10px] rounded-[10px] px-[10px] py-2 text-left text-[14px] whitespace-nowrap transition-colors duration-150 ease-glide focus-visible:outline-2 focus-visible:outline-accent-text ${
                  isActive
                    ? "font-semibold text-side-ink"
                    : "font-medium text-side-ink-2 hover:bg-side-active hover:text-side-ink"
                }`}
              >
                <Icon size={18} strokeWidth={1.7} className="shrink-0" />
                <span className="truncate" title={t(section.labelKey)}>
                  {t(section.labelKey)}
                </span>
              </button>
            </React.Fragment>
          );
        })}
      </nav>
    </aside>
  );
};
