# roundtrip-depth

Compares `translatex roundtrip` configurations on three measures: how much the text is rewritten, how many structural AI tells survive, and how many facts go missing.

## What it measures

| Measure | Definition |
| --- | --- |
| Overlap | Content-word Jaccard between source and result, stop words removed. Lower means more rewriting. |
| Tells left | Texts (of 10) in the result that still contain a structural AI tell: "not just/only X", "more than just", "it's worth noting", "in today's", "at its core", 不仅, 更是, 值得注意, 当今. |
| Facts missing | Names, numbers, IDs and code spans the fact guard (`missingFacts`) reports as absent, summed over the corpus. |
| Latency | Mean wall clock per text for the whole chain. |

`corpus.jsonl` holds 10 texts: 5 AI-flavoured English paragraphs (`kind: ai`), 3 fact-heavy workplace English texts with names, numbers, IDs and code, and 2 Chinese texts. Every run passes `--preserve @code`.

## Run it

```bash
python benches/roundtrip-depth/measure.py --bin target/release/translatex.exe --out out
python benches/roundtrip-depth/measure.py --bin target/release/translatex.exe --out out --configs mt,llm-opencode
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `--bin` | `translatex` on `PATH` | Binary to test |
| `--out` | `benches/roundtrip-depth/out` | Directory for `results.jsonl` (one record per text and configuration) and `summary.md` |
| `--configs` | all | Comma-separated names from `CONFIGS` in `measure.py`: `mt-d1`, `mt` (the default, 2 hops), `mt-deep`, `mt-deep-mixed`, `llm-claude`, `llm-opencode`, `lynote-claude`, `lynote-opencode`, `mt-fix-claude`, `lynote-fix-claude`, `lynote-fix-opencode`, `mt-then-llm-claude`, `mt-then-llm-opencode`, and with `--scorer` also `llm-best3` (`--preset llm --best-of 3`) and `llm-target` (`--preset llm --target-score`) |
| `--sleep` | 0.4 | Seconds between calls, to stay polite to the free endpoints |
| `--timeout` | 600 | Per-call timeout in seconds |
| `--scorer` | none | Detector command (text on stdin, score 0-1 on the last stdout line). Scores source and output of each row, adds "detector src" and "detector out" means to `summary.md`, fills rows resumed without scores, and is passed to `llm-best3` and `llm-target` |
| `--target` | 0.5 | `--target-score` for `llm-target` |

The `claude-*` and `opencode-*` configurations spend quota on your Claude subscription and your OpenCode plan. The `mt` configurations are free.

## Results

Run on 2026-11-11. 10 texts, one run per configuration.

| Configuration | Overlap | Tells left (of 10) | Facts missing | Latency |
| --- | --- | --- | --- | --- |
| mt, 1 hop (old default) | 0.556 | 7 | 5 | 0.5 s |
| mt, 2 hops (new default) | 0.520 | 5 | 5 | 0.9 s |
| mt-deep, 3 hops | 0.480 | 6 | 8 | 1.1 s |
| llm via opencode-cli (DeepSeek) | 0.446 | 0 | 0 | 14 s |
| llm via claude-cli (Haiku) | 0.535 | 0 | 0 | 8 s |
| lynote via claude-cli / opencode-cli | 0.292 / 0.307 | 1 / 0 | 5 / 3 | 19 / 21 s |
| lynote-fix via claude-cli / opencode-cli | 0.369 / 0.396 | 0 / 1 | 1 / 1 | 24 / 28 s |

Reading the table:

- Machine translation changes words and keeps sentence structure, so "not just X, it's Y" survives at every depth.
- A third machine-translation hop loses more facts (8 against 5) than it gains in rewriting.
- LLM hops remove the structural tells and kept every fact on this corpus.
- The repair hop brings `lynote` from 3 to 5 missing facts down to 1 and keeps most of the rewrite.

## Caveats

- 10 texts and one run per configuration: a small sample, and LLM output varies between runs.
- The tell list and the fact guard are heuristics. A tell outside the list is not counted, and the guard checks names only when both texts are in Latin script.
- Overlap rewards change, not quality. A lower number can mean a worse rewrite.
- Without `--scorer` nothing here measures AI detectors. With it, the columns are one detector's scores on this corpus; report them per detector. No result supports a claim of undetectable text.
