# prompt-lab

Experiments on the LLM humanize brief (`build_prompt` in `src/providers/cli_llm.rs`), scored by an English AI-text detector. For tuning only: not-a-bot never runs a detector on live requests.

- `corpus.jsonl`: 30 AI-style English workplace texts (customer email, Slack update, marketing blurb, documentation, LinkedIn post) with the fact tokens each must keep. `python gen_corpus.py` regenerates it from cache.
- `lab.py`: prompt variants as hop lists (`VARIANTS`), cached `claude -p` / `opencode run` calls, one detector batch per run (desklib/ai-text-detector-v1.01 via `wei-harness/humanizer/evals/detect.py`), exact fact-token retention, and a haiku judge (`same_meaning`, `reads_human`, `added_facts`).
- `python lab.py run [--variants a,b] [--limit N]`, then read `results/summary.md`. Run one detector process at a time.

## Findings so far (30 texts, one run per variant)

| variant | mean p_ai | <0.5 | fact loss | same_meaning |
|---|---|---|---|---|
| v0-current, sonnet | 0.736 | 20% | 1/30 | 4.17 |
| v0-oc, deepseek (production default) | 0.530 | 53% | 2/30 | 4.07 |
| v10-oc-light, deepseek | 0.410 | 60% | 4/30 | 3.77 |

- The model matters more than any wording change tried: the same brief scores 0.736 on sonnet and 0.530 on deepseek-v4.1-flash.
- High-scoring outputs keep the source's content-free sentences (thanks, excitement, pledges, generic lessons). Listing human-writing features, a ja pivot, a literal first hop or a haiku final hop all scored worse than the baseline.
- Deleting feeling and pledge sentences (v10, now in `build_prompt`) lowers the score; a broader cut (v6) lowers it the same but drops place names that sat inside the cut sentences.

## Chains (30 texts x 2 samples, real binary)

Detector desklib/ai-text-detector-v1.01, mean P(AI); source texts score 0.999. Fact loss is the number of texts (of 30) missing at least one exact fact token. Meaning is judged per text by Claude Sonnet: the share of texts with no MAJOR meaning error.

| chain | mean P(AI) | fact loss | no major meaning error |
|---|---|---|---|
| `standard` (LLM out, free LLM back, fix) | 0.364 | 0.5 | 98% |
| 3 LLM hops zh->ja->en + fix | 0.380 | 1.5 | 98% |
| LLM out, LLM back given the original, fix (earlier draft) | 0.393 | 0.5 | 98% |
| `light` (Google out, fix, LLM back, fix) | 0.489 | 0 | 97% |
| old mixed chain (LLM zh, LLM ja, Google fi, Edge en) with a fix after each machine hop | 0.494 | 7.0 | 93% |
| same old mixed chain with one final fix | 0.383 | 7.5 | 54% |
| Google out, LLM back, one final fix (no fix after the machine hop) | 0.515 | 0 | 87% |
| LLM out, Google back, fix | 0.642 | 5.5 | 87% |
| 2 machine-translation hops + final fix | 0.930 | 5.0 | 80% |
| 0.1.x standard (LLM out/back, fact-only fix) | 0.387 | 1.3 | 96% (3 samples) |

- Machine translation changes words but keeps sentence structure, so machine-only chains stay near the source score even with repairs. The score drop comes from the LLM rewrite hop.
- A fix right after a machine hop is needed when no LLM hop with the original follows soon: for the Google-out chain it lifts the meaning gate from 87% to 97%.
- Mixed chains whose last hops are machine translation fail the meaning gate.
- No chain that costs more than `standard` scored lower, so there are two presets, `light` and `standard`.
- Sampling noise is large: the same text and variant move several points between samples, so compare pooled means over 2 or more samples, not single runs.
- A fix hop that sees the original and is not limited to facts can restore the feeling and pledge sentences the humanize hops deleted, which raises the score. The shipped fix hop restores meaning only, never cut pleasantries, and caps non-fact edits with one copy-back budget.

## Meaning gate

`python lab.py meaning` judges every output for meaning errors (Claude Sonnet, per text). Environment switches:

- `LAB_WORKERS`: number of parallel judge calls.
- `LAB_HAIKU_JUDGE=0`: skip the 1-5 haiku judge (`same_meaning`, `reads_human`, `added_facts`); the meaning gate still runs.
- `LAB_MEANING=0`: defer the meaning gate (Claude quota) from a run to a later `python lab.py meaning`.

Screening round: run new variants with `--limit 12 --results results-screen` first, then run the survivors on all 30 texts.
