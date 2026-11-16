import type { PaletteId } from "@/lib/themes";

/** A miniature settings window painted by the real palette tokens. */
export function PalettePreview({ palette }: { palette: PaletteId }) {
  return (
    <span
      data-palette={palette}
      aria-hidden="true"
      className="flex h-16 overflow-hidden rounded-lg bg-[var(--bg)]"
    >
      <span className="flex w-1/4 flex-col gap-1 bg-side p-2">
        <span className="h-1 w-full rounded bg-side-ink" />
        <span className="h-1 w-3/4 rounded bg-side-ink-2" />
      </span>
      <span className="flex flex-1 flex-col gap-2 p-3">
        <span className="h-1.5 w-2/3 rounded bg-[var(--ink)]" />
        <span className="h-1 w-full rounded bg-[var(--line-strong)]" />
        <span className="h-3 w-1/3 rounded bg-accent" />
      </span>
    </span>
  );
}
