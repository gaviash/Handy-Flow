// Renders the Handy Flow app icon for every palette.
//   bun scripts/render-icons.ts
// Writes src-tauri/icons/source/handy-flow.svg (the master, with {{bg}} {{fg}}
// {{dot}} placeholders), a 1024px PNG per palette in icons/source/ (feed
// rose.png to `bunx tauri icon`) and a 256px PNG per palette in
// resources/icons/ (the window icon at runtime). Uses the installed Chrome.
import { chromium } from "playwright";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

/** Light-mode icon colors, from the spec's palette table. */
export const ICON_COLORS: Record<string, { bg: string; fg: string; dot: string }> = {
  rose: { bg: "#f28cbb", fg: "#3b1d2c", dot: "#1c1915" },
  sauge: { bg: "#12443d", fg: "#f5f3e8", dot: "#f39ac2" },
  prune: { bg: "#2a1f33", fg: "#f6a5c8", dot: "#c9b6f2" },
  encre: { bg: "#111111", fg: "#fafaf8", dot: "#f28cbb" },
  ocean: { bg: "#0f2a4a", fg: "#eaf1fb", dot: "#6ee7d8" },
  ambre: { bg: "#c8582c", fg: "#fff7ee", dot: "#22180f" },
  lime: { bg: "#141611", fg: "#c6ff3d", dot: "#ffffff" },
  contraste: { bg: "#000000", fg: "#ffd84a", dot: "#ffffff" },
};

function masterSvg(): string {
  const tsx = readFileSync(join(ROOT, "src/components/icons/HandyHand.tsx"), "utf8");
  const d = tsx.match(/d="([^"]+)"/)![1];
  const size = 1024;
  const scale = ((size * 0.62) / 135).toFixed(4); // hand is 62% of the tile
  const r = size * 0.08; // dot: 16% wide
  const inset = size * 0.15 + r; // 15% from the top and right edges
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}">
  <rect width="${size}" height="${size}" rx="${size * 0.23}" fill="{{bg}}"/>
  <g transform="translate(${size / 2} ${size * 0.515}) rotate(-8) scale(${scale}) translate(-63 -67.5)">
    <path fill-rule="evenodd" clip-rule="evenodd" fill="{{fg}}" stroke="{{fg}}" stroke-width="4" d="${d}"/>
  </g>
  <circle cx="${size - inset}" cy="${inset}" r="${r}" fill="{{dot}}"/>
</svg>
`;
}

const fill = (svg: string, c: { bg: string; fg: string; dot: string }) =>
  svg.replaceAll("{{bg}}", c.bg).replaceAll("{{fg}}", c.fg).replaceAll("{{dot}}", c.dot);

if (import.meta.main) {
  const svg = masterSvg();
  const srcDir = join(ROOT, "src-tauri/icons/source");
  const resDir = join(ROOT, "src-tauri/resources/icons");
  mkdirSync(srcDir, { recursive: true });
  mkdirSync(resDir, { recursive: true });
  writeFileSync(join(srcDir, "handy-flow.svg"), svg);

  const browser = await chromium.launch({ executablePath: CHROME });
  const page = await browser.newPage();
  for (const [id, colors] of Object.entries(ICON_COLORS)) {
    for (const [px, file] of [
      [1024, join(srcDir, `${id}.png`)],
      [256, join(resDir, `palette-${id}.png`)],
    ] as const) {
      await page.setViewportSize({ width: px, height: px });
      const html = `<html><body style="margin:0;background:transparent">${fill(svg, colors).replace(
        /width="1024" height="1024"/,
        `width="${px}" height="${px}"`,
      )}</body></html>`;
      await page.setContent(html);
      await page.locator("svg").screenshot({ path: file, omitBackground: true });
    }
    console.log(`rendered ${id}`);
  }
  await browser.close();
}
