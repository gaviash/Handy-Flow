import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

const DAY = 86_400;
const now = () => Math.floor(Date.now() / 1000);

test("the sidebar highlight glides to the chosen section", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  const pill = page.getByTestId("sidebar-pill");
  const top = () => pill.evaluate((el) => el.getBoundingClientRect().top);
  const general = await top();
  await page.getByRole("button", { name: "History", exact: true }).click();
  const history = await page
    .getByRole("button", { name: "History", exact: true })
    .evaluate((el) => el.getBoundingClientRect().top);
  await expect.poll(top).toBeCloseTo(history, 0);
  expect(history).toBeGreaterThan(general);
});

test("a fresh learned word glows, an old one fades", async ({ page }) => {
  await mockTauri(page, {
    results: {
      get_learned_words: [
        { term: "Kubernetes", heard: ["cube"], count: 1, last_seen: now() },
        {
          term: "Qwen",
          heard: ["queen"],
          count: 1,
          last_seen: now() - 27 * DAY,
        },
      ],
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Vocabulary", exact: true }).click();
  await page.getByRole("button", { name: /^Learned words/i }).click();
  await expect(page.getByText("Kubernetes", { exact: true })).toHaveClass(
    /animate-learned/,
  );
  const qwen = page.getByText("Qwen", { exact: true });
  await expect(qwen).not.toHaveClass(/animate-learned/);
  const opacity = await qwen.evaluate((el) =>
    Number(getComputedStyle(el.parentElement!).opacity),
  );
  expect(opacity).toBeLessThan(1);
  await expect(page.locator("li", { has: qwen })).toHaveAttribute(
    "title",
    /Forgotten on/,
  );
});

test("the empty history greets with the mascot", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(page.locator(".animate-wave")).toBeVisible();
});

test("a snippet previews what saying it types", async ({ page }) => {
  await mockTauri(page, {
    settings: { snippets: [{ trigger: "signature", text: "Marie Dupont" }] },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Vocabulary", exact: true }).click();
  await page.getByRole("button", { name: /^Snippets/ }).click();
  await page.getByRole("button", { name: "Preview", exact: true }).click();
  await expect(page.getByText("Say “signature” on its own")).toBeVisible();
  await expect(page.locator(".snippet-reveal")).toHaveText("Marie Dupont");
});

test("the style list shows an example of the tone", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Enhancement", exact: true }).click();
  await expect(page.getByText("Example for Email · Formal")).toBeVisible();
  await expect(
    page.getByText(/I confirm our meeting tomorrow at 10 a\.m\./),
  ).toBeVisible();
});

const entry = (id: number, raw: string, rewritten: string | null) => ({
  id,
  file_name: `handy-${id}.wav`,
  timestamp: now(),
  saved: false,
  title: "",
  transcription_text: raw,
  post_processed_text: rewritten,
  post_process_prompt: null,
  post_process_requested: rewritten !== null,
  source_name: null,
  language: null,
  language_source: null,
  app: null,
});

test("History shows what the rewrite changed", async ({ page }) => {
  await mockTauri(page, {
    results: {
      get_history_entries: {
        entries: [
          entry(
            1,
            "euh bonjour je voulais dire merci",
            "Bonjour, je voulais dire merci.",
          ),
        ],
        has_more: false,
      },
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "See the edits" }).click();
  await expect(page.locator(".diff-removed")).toHaveText("euh bonjour");
  await expect(page.locator(".diff-added").first()).toHaveText("Bonjour,");
});

test("a dictation landing in History unfolds at the top", async ({ page }) => {
  const mock = await mockTauri(page, {
    results: {
      get_history_entries: {
        entries: [entry(1, "premier", null)],
        has_more: false,
      },
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(page.getByText("premier")).toBeVisible();
  await mock.emit("history-update-payload", {
    action: "added",
    entry: entry(2, "deuxième", null),
  });
  await expect(page.getByText("deuxième")).toBeVisible();
  // The class goes once unfolded, so it never clips the row's menus.
  await expect(page.locator(".history-arrive")).toHaveCount(0);
});

test("coming back from the tray replays the page's arrival", async ({
  page,
}) => {
  const mock = await mockTauri(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "General", level: 1 }),
  ).toBeVisible();
  await page.waitForTimeout(500); // let the first arrival finish
  await mock.emit("main-window-shown", null);
  expect(
    await page.evaluate(() => document.getAnimations().length),
  ).toBeGreaterThan(0);
});

test("the pill names the register the rewrite writes in", async ({ page }) => {
  const backend = await mockTauri(page, { label: "recording_overlay" });
  await page.goto("/src/overlay/index.html");
  await expect
    .poll(async () =>
      (await backend.calls()).some((c) => c.cmd === "get_app_settings"),
    )
    .toBe(true);
  await backend.emit("show-overlay", "recording");
  await backend.emit("rewrite-register", ["email", "formal"]);
  await backend.emit("show-overlay", "processing");
  await expect(page.locator(".swork-label")).toHaveText("Email · Formal");
  // A new dictation forgets the last register.
  await backend.emit("show-overlay", "recording");
  await backend.emit("show-overlay", "processing");
  await expect(page.locator(".swork-label")).toHaveText("Processing...");
});

/** A mono 16 kHz WAV, loud then quiet, so the waveform has a shape. */
function wav(): Buffer {
  const rate = 16_000;
  const n = rate; // one second
  const buf = Buffer.alloc(44 + n * 2);
  buf.write("RIFF", 0);
  buf.writeUInt32LE(36 + n * 2, 4);
  buf.write("WAVEfmt ", 8);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(1, 22);
  buf.writeUInt32LE(rate, 24);
  buf.writeUInt32LE(rate * 2, 28);
  buf.writeUInt16LE(2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write("data", 36);
  buf.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) {
    const amp = i < n / 2 ? 20_000 : 2_000;
    buf.writeInt16LE(Math.round(amp * Math.sin(i / 8)), 44 + i * 2);
  }
  return buf;
}

test("a recording shows its own waveform", async ({ page }) => {
  await mockTauri(page, {
    results: {
      get_audio_file_path: "voice.wav",
      get_history_entries: {
        entries: [entry(1, "bonjour", null)],
        has_more: false,
      },
    },
  });
  await page.route("**/voice.wav", (route) =>
    route.fulfill({ body: wav(), contentType: "audio/wav" }),
  );
  await page.addInitScript(() => {
    HTMLMediaElement.prototype.play = function () {
      this.dispatchEvent(new Event("play"));
      return Promise.resolve();
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "Play audio" }).click();
  const bars = page.getByTestId("waveform").locator("i");
  await expect(bars).toHaveCount(40);
  const h = (i: number) =>
    bars.nth(i).evaluate((el) => parseFloat((el as HTMLElement).style.height));
  expect(await h(5)).toBeGreaterThan(await h(35)); // loud start, quiet end
  // Still a real slider for the keyboard.
  await expect(
    page.getByRole("slider", { name: "Playback position" }),
  ).toHaveCount(1);
});
