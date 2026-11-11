# prompt-lab

Experiments on the LLM humanize brief (`build_prompt` in `src/providers/cli_llm.rs`), scored by an English AI-text detector. For tuning only: translatex never runs a detector on live requests.

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
- Checked with the real binary on `benches/roundtrip-depth` (8 English texts, desklib): `standard` 0.657 to 0.611, `strong` 0.711 to 0.697, no missing facts before or after.
