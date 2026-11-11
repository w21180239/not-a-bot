# Detector (dev-time only)

`detect.py` scores texts with the open AI-text classifier `desklib/ai-text-detector-v1.01` (DeBERTa-v3-large, max 768 tokens; CUDA if available, then MPS, else CPU) and reports `p_ai`. It is a noisy relative signal for comparing variants, never a pass/fail gate, and neither the skill nor the CLI calls it at runtime. Scores climb with length and short texts sit near 0, so compare within word buckets and against a human-written control. `calibration.md` has the method and numbers: AUROC 0.88 on English, about chance on Chinese.

## Requirements

Python 3.10+, `torch`, `transformers<5` (version 5 breaks the model's custom class), `sentencepiece`, `protobuf`. The first run downloads about 1.6 GB of weights. The model holds 1-2 GB of RAM; run one scorer at a time. Nothing here is installed by the Rust build or the npm package.

```sh
# from the repo root, once; .venv is git-ignored
python3 -m venv benches/detector/.venv
benches/detector/.venv/bin/pip install torch 'transformers<5' sentencepiece protobuf
```

Windows (CPU only, `uv`):

```sh
uv venv benches/detector/.venv
uv pip install --python benches/detector/.venv/Scripts/python.exe torch --index-url https://download.pytorch.org/whl/cpu
uv pip install --python benches/detector/.venv/Scripts/python.exe "transformers<5" sentencepiece protobuf
```

## Use

```sh
# paths are relative to the repo root; on Windows use .venv\Scripts\python.exe
benches/detector/.venv/bin/python benches/detector/detect.py benches/skill-evals/runs/<ts>   # writes detector.jsonl in that run dir
benches/detector/.venv/bin/python benches/detector/detect.py --files a.txt b.txt            # ad-hoc scoring
python benches/skill-evals/grade.py benches/skill-evals/runs/<ts> --detector-only            # adds the detector table to summary.md
```

## Stdin scorer

`scorer.cmd` (Windows) and `scorer.sh` run the venv python on `detect.py --stdin` with no shell needed. They read UTF-8 from stdin, log to stderr and print `p_ai` (4 decimals) as the last stdout line. Each call reloads the model.

```sh
echo "some text" | benches/detector/scorer.sh
nab rewrite "<text>" --from en --preset standard --scorer benches/detector/scorer.sh --best-of 3 --format json
```

`detect.py --batch in.jsonl [--out out.jsonl]` scores many rows (a `text` field; adds `p_ai`) in one model load.

If `detector-human.jsonl` (same format) sits in a run directory, `grade.py` adds a `human-control` row. Keep human-written control texts outside the repo and commit only scores.
