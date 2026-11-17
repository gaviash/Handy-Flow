/** A run of text from either side of a rewrite: kept, removed or added. */
export type DiffPart = { text: string; kind: "same" | "removed" | "added" };

// Words and punctuation apart, each with the spaces before it, so a comma
// added after a word shows as just the comma.
const TOKEN =
  /\s*(?:[\p{L}\p{M}\p{N}][\p{L}\p{M}\p{N}'’-]*|[^\s\p{L}\p{M}\p{N}])/gu;
const tokens = (s: string) => s.match(TOKEN) ?? [];

// shortcut: plain LCS, quadratic; past this many cells it gives up rather
// than freeze the page on a very long dictation.
const MAX_CELLS = 2_000_000;

/** What a rewrite changed, word by word, or null when the texts are too long
 * or too reworded for a word-by-word view to mean anything. */
export function wordDiff(before: string, after: string): DiffPart[] | null {
  const a = tokens(before);
  const b = tokens(after);
  if (a.length * b.length > MAX_CELLS) return null;
  const key = (t: string) => t.trim();
  // lcs[i][j] = longest common run of a[i..] and b[j..].
  const lcs = Array.from(
    { length: a.length + 1 },
    () => new Uint16Array(b.length + 1),
  );
  for (let i = a.length - 1; i >= 0; i--)
    for (let j = b.length - 1; j >= 0; j--)
      lcs[i][j] =
        key(a[i]) === key(b[j])
          ? lcs[i + 1][j + 1] + 1
          : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
  const kept = lcs[0][0];
  if (kept < 0.4 * Math.max(a.length, b.length)) return null;

  const parts: DiffPart[] = [];
  const push = (text: string, kind: DiffPart["kind"]) => {
    const last = parts[parts.length - 1];
    if (last?.kind === kind) last.text += text;
    else parts.push({ text, kind });
  };
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && key(a[i]) === key(b[j])) {
      push(b[j], "same");
      i++;
      j++;
    } else if (
      j < b.length &&
      (i === a.length || lcs[i][j + 1] > lcs[i + 1][j])
    ) {
      push(b[j++], "added");
    } else {
      push(a[i++], "removed");
    }
  }
  return parts;
}
