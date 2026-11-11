# roundtrip-depth

Compares `translatex roundtrip` configurations on three measures: how much the text is rewritten, how many structural AI tells survive, and how many facts go missing.

## What it measures

| Measure | Definition |
| --- | --- |
| Overlap | Content-word Jaccard between source and result, stop words removed. Lower means more rewriting. |
| Tells left | Texts (of 10) in the result that still contain a structural AI tell: "not just/only X", "more than just", "it's worth noting", "in today's", "at its core", 不仅, 更是, 值得注意, 当今. |
| Facts missing | Names, numbers, IDs and code spans the fact guard (`missingFacts`) reports as absent, summed over the corpus. |
| Latency | Mean wall clock per text for the whole chain. |

`corpus.jsonl` holds 10 texts: 5 AI-flavoured English paragraphs (`kind: ai`), 3 fact-heavy workplace English texts with names, numbers, IDs and code, and 2 Chinese texts. Runs cover English only unless `--langs en,zh` adds the Chinese texts; the results below used all 10. Every run passes `--preserve @code`.

## Run it

```bash
python benches/roundtrip-depth/measure.py --bin target/release/translatex.exe --out out
python benches/roundtrip-depth/measure.py --bin target/release/translatex.exe --out out --configs light,standard-opencode
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `--bin` | `translatex` on `PATH` | Binary to test |
| `--out` | `benches/roundtrip-depth/out` | Directory for `results.jsonl` (one record per text and configuration) and `summary.md` |
| `--configs` | all | Comma-separated names from `CONFIGS` in `measure.py`: `light-d1` (`--depth 1`), `light` (the default preset, 2 hops), `standard-claude`, `standard-opencode`, `strong-claude`, `strong-opencode`, the by-hand chains `light-fix-claude` and `light-then-llm-claude` / `light-then-llm-opencode`, and with `--scorer` also `standard-best3` (`--preset standard --best-of 3`) and `standard-target` (`--preset standard --target-score`) |
| `--langs` | `en` | Comma-separated corpus languages to run (`en`, `zh`). `summary.md` opens with the languages it covers. Rows already in `results.jsonl` for other languages stay in the file and are left out of the summary |
| `--sleep` | 0.4 | Seconds between calls, to stay polite to the free endpoints |
| `--timeout` | 600 | Per-call timeout in seconds |
| `--scorer` | none | Detector command (text on stdin, score 0-1 on the last stdout line). Scores source and output of each row, adds "detector src" and "detector out" means to `summary.md`, fills rows resumed without scores, and is passed to `standard-best3` and `standard-target` |
| `--target` | 0.5 | `--target-score` for `standard-target` |
| `--batch-scorer` | none | Detector command that takes `--batch IN.jsonl --out OUT.jsonl` and adds `p_ai` to each row, such as `<venv python> wei-harness/humanizer/evals/detect.py`. Loads the model once after the runs and fills the detector columns in place of one `--scorer` call per text; `--scorer` is then only passed to `standard-best3` and `standard-target` |
| `--parallel` | 1 | Texts run concurrently. 4 suits the LLM configurations; keep 1 for long `light` runs against the free endpoints |

The `claude-*` and `opencode-*` configurations spend quota on your Claude subscription and your OpenCode plan. The `light` configurations are free.

## Results

Run on 2026-11-11. 10 texts, one run per configuration.

| Configuration | Overlap | Tells left (of 10) | Facts missing | Latency |
| --- | --- | --- | --- | --- |
| light, 1 hop (`--depth 1`, 0.1.x default) | 0.556 | 7 | 5 | 0.5 s |
| light, 2 hops (0.3 default) | 0.520 | 5 | 5 | 0.9 s |
| 3 machine-translation hops (`--depth 3`) | 0.480 | 6 | 8 | 1.1 s |
| standard via opencode-cli (DeepSeek) | 0.446 | 0 | 0 | 14 s |
| standard via claude-cli (Haiku) | 0.535 | 0 | 0 | 8 s |
| strong via claude-cli / opencode-cli | 0.369 / 0.396 | 0 / 1 | 1 / 1 | 24 / 28 s |

Reading the table:

- Machine translation changes words and keeps sentence structure, so "not just X, it's Y" survives at every depth.
- A third machine-translation hop loses more facts (8 against 5) than it gains in rewriting.
- LLM hops remove the structural tells and kept every fact on this corpus.
- The repair hop brings the same chain without it (earlier run, by hand) from 3 to 5 missing facts down to 1 and keeps most of the rewrite; that chain with the repair hop is `strong`.

## Caveats

- 10 texts and one run per configuration: a small sample, and LLM output varies between runs.
- The tell list and the fact guard are heuristics. A tell outside the list is not counted, and the guard checks names only when both texts are in Latin script.
- Overlap rewards change, not quality. A lower number can mean a worse rewrite.
- Without `--scorer` nothing here measures AI detectors. With it, the columns are one detector's scores on this corpus; report them per detector. No result supports a claim of undetectable text.
