import assert from "node:assert/strict";
import { wordDiff } from "./wordDiff";

const show = (a: string, b: string) =>
  wordDiff(a, b)
    ?.map((p) =>
      p.kind === "same"
        ? p.text
        : p.kind === "added"
          ? `[+${p.text}]`
          : `[-${p.text}]`,
    )
    .join("");

// Fillers removed, punctuation and capital added.
assert.equal(
  show("euh bonjour je voulais dire merci", "Bonjour, je voulais dire merci."),
  "[-euh bonjour][+Bonjour,] je voulais dire merci[+.]",
);
// Same text: nothing marked.
assert.equal(show("ok merci", "ok merci"), "ok merci");
// Fully reworded: no word-by-word view.
assert.equal(wordDiff("un deux trois quatre", "alpha beta gamma delta"), null);

console.log("wordDiff: ok");
