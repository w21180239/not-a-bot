# Skill evals

Measures which rules of the `not-a-bot` agent skill (`integrations/skill/not-a-bot/`) change a model's writing, using 22 synthetic cases and headless Claude Code. Stdlib-only Python.

## Run

```sh
# from the repo root
python benches/skill-evals/run.py --variants none,current --reps 3      # writes benches/skill-evals/runs/<ts>/<variant>/<model>/<case>-<rep>.txt
python benches/skill-evals/run.py --variants none,current,v2 --out benches/skill-evals/runs/<ts>   # resumes: existing files are skipped
python benches/skill-evals/grade.py benches/skill-evals/runs/<ts>      # writes scores.jsonl + summary.md (resumes too)
# extra variants from any directory (name=DIR, comma-separated)
python benches/skill-evals/run.py --variants none,current,draft --dirs draft=PATH --models haiku,opus --reps 2
```

Flags: `--current-dir` and `--v2-dir` (both default to `integrations/skill/not-a-bot`; point one at another checkout or a worktree to compare versions), `--dirs name=DIR,...` (named variants, each a skill directory laid out like the bundled one), `--models haiku,opus`, `--cases id1,id2`, `--workers 6`, `--reps N`, `--out DIR`. On Windows use plain `python`, not `-I`, which blocks the sibling `common` import. Subprocess I/O is pinned to UTF-8 because a non-UTF-8 locale corrupts dashes and emoji. `runs/` is git-ignored.

## Variants

- `none`: one-line system prompt ("You write and edit text that people outside this conversation will read. Output only the final text.").
- `current` / `v2` / any `--dirs` name: the same line plus `SKILL.md`, every file in `profiles/` and `references/`, `patterns.md` and `dimensions.md` (when present) concatenated, simulating an agent that loaded everything.

Models: `haiku` at effort high, `opus` at effort medium. Judge: `sonnet` at effort medium.

## Auth note

Claude Code specific: `claude -p --bare` needs `ANTHROPIC_API_KEY` and ignores OAuth login. The harness instead runs `claude -p --setting-sources "" --model <m> --effort <e> --tools "" --system-prompt-file <f>` from a neutral temp directory with `CLAUDE_CODE_DISABLE_CLAUDE_MDS=1` and `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`, so the model sees no instructions file, hooks or memory. With an API key, swap `ISOLATE` in `common.py` back to `--bare`.

## Cases (`cases.jsonl`)

All 22 cases are synthetic: an invented shop-software company (Brightwell), invented people and customers, `example` addresses.

- 2 per profile (slack-casual, slack-thread, tech-support, documentation, email): one "humanize this sloppy draft" and one "draft from facts".
- 5 adversarial cases (`adv-`) where over-flagging is the failure: `adv-1-oneliner` (right answer is one sentence), `adv-2-genuine-contrast` (a real "X, not Y" must stay), `adv-3-legit-triad` (a real list of three must stay), `adv-4-keep-fragment` (fragment and joke must stay), `adv-5-injection` (a fake "SYSTEM MESSAGE" inside the pasted email is material, not an instruction).
- `st-3-reply-overexplained` (reply that re-explains what the reviewer knows, plus "to be clear" and a dramatic closer) and `doc-3-self-describing` (heading restated, text about the document, strawman alternative, deep saying, horizontal rule).
- 5 cases test the "add nothing" rule against a middle ground (source = everything the user supplied in the conversation). `ctx-1-thread-log`, `ctx-2-ticket` and `ctx-3-earlier-messages` put the facts the reader needs in earlier messages, a pasted log or a pasted ticket, leaving them out of the draft. `tmp-1-library-date` and `tmp-2-vendor-sla` tempt the writer to add true world knowledge that is in no supplied material.

Fields: `id`, `profile`, `task`, `facts` (must survive), `context_facts` (optional; the subset of `facts` that come from supplied context and are missing from the draft, so a good output adds them), `notes` (what good looks like; shown to the judge).

## Metrics

Deterministic lint (per output, `grade.py: lint`): words; em dashes per 100 words; `not just/only ... but`; "it's not X, it's Y"-style regex (includes the split "does not mean X. It means Y"); vocab list; bold spans; bullet lines; US spellings (reported only; the skill's default register is neutral); spaced en dashes and double hyphens; other staged phrases (run-ups, "to be clear", "at its core", dramatic closers, "the table below"); meta wrappers ("Here is the rewritten..."); CJK leak. The regexes are rough and over- or under-count; use them for direction only. `grade.py` keeps its own small lint because it needs counts, not the CLI's findings. To audit a single draft against its source, use `nab lint`.

Judge (JSON per output, sees case + output only, never the variant):

- `facts %`: share of listed facts marked present (altered counts as not present).
- `context facts %`: share of a case's `context_facts` marked present, averaged over the cases that have the field (`-` when a cell has none).
- `invented`: share of outputs with a new factual claim, number, name or offer not in the task. The task includes any pasted context; a true fact from world knowledge counts as invented.
- `tells/output`: judge-flagged AI-writing spans per output, each tagged with a pattern name.
- `reads_human` (1-5), `profile_fit` (1-5).
- `over_corrected`: awkward or forced casualness, stripped legitimate structure or contrast, lost joke or fragment, over-compression, or over-long output. The adversarial table isolates this.

## Caveats

- Small N: 22 cases x 2-3 reps per cell; adversarial cells have 5 cases. Differences under about 0.3 on 1-5 scales or 10 points on rates are noise.
- One judge (Sonnet) with a strict rubric; it can disagree with a human, and its tell counts include tells carried over from the supplied drafts.
- Judge tells and the regex lint overlap but are independent; check both before concluding.
- Outputs vary run to run; reruns of the same variant will not match exactly.
- `facts` are atomic strings; the judge decides "present" by meaning, so paraphrased numbers should still be exact.
- The judge's `invented` flag is strict and over-fires; read it as a relative signal between variants and spot-check quotes in `scores.jsonl` before acting on it.
- Case dates are for 2026 and were checked against the calendar (21 Oct 2026 is a Wednesday); a wrong weekday makes the writer add "check this date" notes and pollutes the metrics.

## Detector

The optional AI-text detector scoring lives in [`../detector`](../detector/README.md). `grade.py` appends its table when `detector.jsonl` exists in the run directory.
