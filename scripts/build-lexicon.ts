/**
 * Builds src-tauri/resources/lexicon/{fr,en}.txt: the 50 000 most frequent
 * words of each language, lowercased, one per line, most frequent first.
 * Source (data CC BY-SA 4.0, Hermit Dave, OpenSubtitles 2018):
 *   https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/fr/fr_50k.txt
 *   https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/en/en_50k.txt
 * Rerun: `bun scripts/build-lexicon.ts`.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";

const BASE =
  "https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018";

for (const lang of ["fr", "en"]) {
  const res = await fetch(`${BASE}/${lang}/${lang}_50k.txt`);
  if (!res.ok) throw new Error(`${lang}: HTTP ${res.status}`);
  const seen = new Set<string>();
  for (const line of (await res.text()).split("\n")) {
    const word = line.split(" ")[0]?.trim().toLowerCase();
    if (word) seen.add(word);
  }
  const out = join(
    import.meta.dir,
    `../src-tauri/resources/lexicon/${lang}.txt`,
  );
  writeFileSync(out, [...seen].slice(0, 50_000).join("\n") + "\n");
  console.log(`${lang}: ${Math.min(seen.size, 50_000)} words`);
}
