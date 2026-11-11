# Detector calibration

Two local AI-text scorers compared on a small labelled set. Both are dev-time relative signals, never gates.

## Setup

| Scorer | Model | Output |
| --- | --- | --- |
| `detect.py` (`scorer.cmd`) | `desklib/ai-text-detector-v1.01` (DeBERTa-v3-large classifier, 768 tokens) | `p_ai` in [0,1] |
| fastdetect (removed after this calibration) | `Qwen/Qwen2.5-0.5B` base, Fast-DetectGPT analytic criterion, sampling model = scoring model, 512 tokens | raw criterion; `score` = logistic(raw) in [0,1] |

Calibration set: 120 texts, 60 questions x (human, AI), 30 English and 30 Chinese.

- Human: the first human answer of 40-200 words (en) / 60-400 characters (zh) from the public HC3 (`Hello-SimpleAI/HC3`) and HC3-Chinese raw jsonl. English sources: finance, medicine, open_qa, reddit_eli5, wiki_csai (6 each). Chinese: baike 5, finance 5, law 4, medicine 4, nlpcc_dbqa 4, open_qa 4, psychology 4. Seeded shuffle (seed 7).
- AI: the same question answered by `claude -p --model haiku --setting-sources "" --tools ""` with "Answer in 2-4 sentences." appended (plus "Answer in Chinese." for zh), run from an empty cwd with `CLAUDE_CODE_DISABLE_CLAUDE_MDS=1` and `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`.
- Length: en human 90 words on average (42-183), en AI 78 (55-109); zh human 153 characters (60-356), zh AI 180 (72-304).
- The set lives outside the repo (human text is third-party data); the repo keeps only the numbers below.

The fastdetect logistic maps raw to score as `1 / (1 + exp(-(raw - MID) / SCALE))`. Logistic regression on raw, pooled over both languages, gave `MID = -0.5369`, `SCALE = 5.4303`. **These are fitted in-sample on the same 120 texts**, so any accuracy figure from them is optimistic.

## Results

AUROC (0.5 = chance; AI = positive class):

| Scorer | en (n=60) | zh (n=60) | all (n=120) |
| --- | --- | --- | --- |
| desklib `p_ai` | 0.880 | 0.429 | 0.672 |
| fastdetect raw | 0.528 | 0.591 | 0.561 |

Mean score per class:

| Scorer | en human | en AI | zh human | zh AI |
| --- | --- | --- | --- | --- |
| desklib `p_ai` | 0.173 | 0.631 | 0.386 | 0.319 |
| fastdetect raw | -0.636 | -0.478 | -0.663 | -0.388 |
| fastdetect score | 0.496 | 0.503 | 0.494 | 0.507 |

Spearman correlation between the scorers (raw vs `p_ai`): en 0.073, zh -0.130, all -0.027. They are effectively uncorrelated here.

In-sample accuracy at 0.5: fastdetect 0.55 (fitted), desklib 0.62 (not fitted).

Reading:

- desklib separates Haiku from HC3 human text well in English and is at or below chance in Chinese (the model is English-trained).
- fastdetect with a 0.5B model is close to chance for Haiku text in both languages. The implementation is sound: texts greedy-decoded from Qwen2.5-0.5B itself score a raw criterion of about +4.1 on average (7 prompts) and temperature-1 samples about -0.3, as the criterion predicts. Haiku text simply sits far from this small model's own distribution. The logistic fit is therefore nearly flat (scale 5.4); do not read its 0.5 midpoint as a verdict.
- Levers if fastdetect is needed: a larger scoring model (`--model Qwen/Qwen2.5-1.5B` or a different sampling/scoring pair, not run), or longer texts (the criterion sums over tokens, and these answers are 2-4 sentences).

## Round-trip probe

10 English AI texts (the first ten en AI rows) through an early build of this tool (then named translatex 0.2.0; the command is now `not-a-bot rewrite --from en --preserve @code --format json`) with an LLM preset (en -> zh -> en). Three texts timed out at the default 180 s per hop and were rerun with `--timeout-ms 600000`.

| Scorer | mean before | mean after | change | texts that dropped |
| --- | --- | --- | --- | --- |
| desklib `p_ai` | 0.621 | 0.441 | -0.181 | 7 / 10 |
| fastdetect raw | -0.866 | -0.671 | +0.195 | 4 / 10 |
| fastdetect score | 0.485 | 0.494 | +0.009 | 4 / 10 |

Round-tripping lowers desklib's `p_ai` by about 0.18 on average, but individual texts range from nearly unchanged (0.95 -> 0.94) to large drops (0.99 -> 0.29). fastdetect moves in the other direction and by less than its per-text spread (raw changes of up to +/-2), so it gives no usable round-trip signal at this model size.

## Caveats

- Small N: 30 pairs per language and 10 round-trip texts. With 30 vs 30, an AUROC standard error is roughly 0.07, so fastdetect's 0.53 and 0.59 are both consistent with chance and the zh desklib 0.43 is not clearly below it.
- One AI source (Haiku, short 2-4 sentence answers, prompted with the HC3 question). Other models, longer texts and different prompts will shift every number.
- HC3 human text is 2022-23 forum, Q&A and encyclopedia style, often terse or list-like (one zh human answer is a bare list of university names). It is easier to tell from a chat model than current human prose is, and differs from the essay-style text the skill works on.
- Human and AI answers differ in length in places; desklib is known to rise with length (see `README.md` here), which can help or hurt AUROC here.
- The logistic is fitted in-sample and pooled over languages; refit on a larger, held-out set before reading `score` as a probability.
