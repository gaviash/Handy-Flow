"""Builds asr-cases.json for src-tauri/tests/asr_context_bench.rs from a Handy
data folder: each saved recording with the text the user kept (else the
rewrite). The output holds real dictations: write it outside the repo.

usage: python -I asr_cases.py <data dir> <out.json> [term ...]
A case lists the given terms that its expected text contains.
"""
import json
import sqlite3
import sys
from pathlib import Path


def build(data: Path, terms: list[str]) -> list[dict]:
    # Read-only: the app may be running.
    db = sqlite3.connect(f"file:{data / 'history.db'}?mode=ro", uri=True)
    cols = {r[1] for r in db.execute("PRAGMA table_info(transcription_history)")}
    kept = "kept_text" if "kept_text" in cols else "NULL"
    rows = db.execute(
        f"SELECT file_name, COALESCE({kept}, post_processed_text, transcription_text) "
        "FROM transcription_history ORDER BY timestamp"
    )
    cases = []
    for file_name, expected in rows:
        wav = data / "recordings" / file_name
        if not wav.exists() or not (expected or "").strip():
            continue
        low = expected.lower()
        cases.append({
            "wav": str(wav),
            "expected": expected,
            "terms": [t for t in terms if t.lower() in low],
        })
    return cases


if __name__ == "__main__":
    data, out, *terms = sys.argv[1:]
    cases = build(Path(data), terms)
    Path(out).write_text(json.dumps(cases, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"{len(cases)} cases, {sum(len(c['terms']) for c in cases)} term occurrences")
