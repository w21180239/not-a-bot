#!/usr/bin/env python3
"""Hold-out AI-text detectors for re-checking text selected with desklib (detect.py).

Text picked by one detector can carry that detector's blind spots, so a selection
result only counts if a detector that took no part in it agrees.

Usage: .venv/bin/python holdout.py --batch rows.jsonl --out scored.jsonl [--model fakespot]
       echo text | .venv/bin/python holdout.py --stdin [--model hc3]   # last stdout line = p_ai
Rows need a "text" field; every other field is copied through, plus "p_ai" and "detector".
"""
import argparse, json, re, sys
from html import unescape

MODELS = {
    # name: (repo, index of the AI label)
    # RoBERTa-base, Fakespot/Mozilla 2025, Apache-2.0.
    "fakespot": ("fakespot-ai/roberta-base-ai-text-detection-v1", 1),
    # RoBERTa-large trained adversarially against a paraphraser (RADAR, NeurIPS 2023); index 0 = AI per its demo.
    "radar": ("TrustSafeAI/RADAR-Vicuna-7B", 0),
    # e5-small LoRA on RAID-train, MIT.
    "e5raid": ("MayZhou/e5-small-lora-ai-generated-detector", 1),
    # RoBERTa-base on HC3 (ChatGPT, 2023). Scores every corpus source as human; kept only as a record.
    "hc3": ("Hello-SimpleAI/chatgpt-detector-roberta", 1),
}


def clean_text(t):
    """Fakespot's recommended preprocessing (utils.py in the model repo): strip markdown, one line."""
    t = re.sub(r"```.*?```", "", t, flags=re.DOTALL)
    t = re.sub(r"`[^`]*`", "", t)
    t = re.sub(r"!\[.*?\]\(.*?\)", "", t)
    t = re.sub(r"\[([^\]]+)\]\(.*?\)", r"\1", t)
    t = re.sub(r"(\*\*|__)(.*?)\1", r"\2", t)
    t = re.sub(r"(\*|_)(.*?)\1", r"\2", t)
    t = re.sub(r"#+ ", "", t)
    t = re.sub(r"^>.*$", "", t, flags=re.MULTILINE)
    t = re.sub(r"^(\s*[-*+]|\d+\.)\s+", "", t, flags=re.MULTILINE)
    t = re.sub(r"^\s*[-*_]{3,}\s*$", "", t, flags=re.MULTILINE)
    t = re.sub(r"\|.*?\|", "", t)
    t = re.sub(r"<.*?>", "", t)
    t = unescape(t)
    for c in "\n\t\r":
        t = t.replace(c, " ")
    t = t.replace(" ,", ",")
    return re.sub(" +", " ", t).strip()


def scorer(name):
    import torch
    from transformers import AutoTokenizer, AutoModelForSequenceClassification
    dev = torch.device("cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu")
    tok = AutoTokenizer.from_pretrained(MODELS[name][0])
    model = AutoModelForSequenceClassification.from_pretrained(MODELS[name][0]).to(dev).eval()

    def score(text):
        enc = tok(clean_text(text) if name == "fakespot" else text, truncation=True, max_length=512,
                  return_tensors="pt").to(dev)
        with torch.no_grad():
            return round(torch.softmax(model(**enc).logits, dim=-1)[0, MODELS[name][1]].item(), 4)
    return score


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", choices=sorted(MODELS), default="fakespot")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--batch")
    g.add_argument("--stdin", action="store_true")
    ap.add_argument("--out")
    a = ap.parse_args()
    score = scorer(a.model)
    if a.stdin:
        print(score(sys.stdin.read()))
        return
    if not a.out:
        sys.exit("--batch needs --out")
    rows = [json.loads(l) for l in open(a.batch, encoding="utf-8") if l.strip()]
    with open(a.out, "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps({**r, "p_ai": score(r["text"]), "detector": MODELS[a.model][0]}, ensure_ascii=False) + "\n")
    print(f"scored {len(rows)} rows with {MODELS[a.model][0]}", file=sys.stderr)


if __name__ == "__main__":
    main()
