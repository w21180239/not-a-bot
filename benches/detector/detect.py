#!/usr/bin/env python3
"""Dev-time AI-text detector (desklib/ai-text-detector-v1.01). Noisy relative signal; never a gate.

Usage: .venv/bin/python detect.py <run_dir>          # writes <run_dir>/detector.jsonl
       .venv/bin/python detect.py --files a.txt b.txt
       echo text | .venv/bin/python detect.py --stdin   # last stdout line = p_ai (4 decimals); logs on stderr
       (Windows: .venv/Scripts/python.exe)
"""
import argparse, glob, json, os, sys

MODEL = "desklib/ai-text-detector-v1.01"
MAX_LEN = 768
_cache = {}


def _load():
    if _cache:
        return _cache["tok"], _cache["model"], _cache["dev"]
    import torch, torch.nn as nn
    from transformers import AutoTokenizer, AutoConfig, AutoModel, PreTrainedModel

    class DesklibAIDetectionModel(PreTrainedModel):
        config_class = AutoConfig

        def __init__(self, config):
            super().__init__(config)
            self.model = AutoModel.from_config(config)
            self.classifier = nn.Linear(config.hidden_size, 1)
            self.init_weights()

        def forward(self, input_ids, attention_mask=None, labels=None):
            outputs = self.model(input_ids, attention_mask=attention_mask)
            last_hidden_state = outputs[0]
            mask = attention_mask.unsqueeze(-1).expand(last_hidden_state.size()).float()
            pooled = (last_hidden_state * mask).sum(dim=1) / torch.clamp(mask.sum(dim=1), min=1e-9)
            return {"logits": self.classifier(pooled)}

    dev = torch.device("cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu")
    tok = AutoTokenizer.from_pretrained(MODEL)
    model = DesklibAIDetectionModel.from_pretrained(MODEL).to(dev).eval()
    _cache.update(tok=tok, model=model, dev=dev)
    return tok, model, dev


def score(texts):
    """Probability each text is AI-written."""
    import torch
    tok, model, dev = _load()
    out = []
    for t in texts:
        enc = tok(t, padding="max_length", truncation=True, max_length=MAX_LEN, return_tensors="pt")
        with torch.no_grad():
            logits = model(enc["input_ids"].to(dev), attention_mask=enc["attention_mask"].to(dev))["logits"]
        out.append(float(torch.sigmoid(logits).item()))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("rundir", nargs="?")
    ap.add_argument("--files", nargs="+")
    ap.add_argument("--stdin", action="store_true")
    ap.add_argument("--batch", metavar="FILE.jsonl", help="rows with \"text\"; adds \"p_ai\"; one model load")
    ap.add_argument("--out", metavar="OUT.jsonl", help="--batch output (default stdout)")
    a = ap.parse_args()
    if a.batch:
        rows = [json.loads(l) for l in open(a.batch, encoding="utf-8-sig") if l.strip()]
        ps = score([r["text"].strip() for r in rows])
        fh = open(a.out, "w", encoding="utf-8") if a.out else sys.stdout
        for r, p in zip(rows, ps):
            r["p_ai"] = round(p, 4)
            fh.write(json.dumps(r, ensure_ascii=False) + "\n")
        if a.out:
            fh.close()
            print(f"wrote {len(rows)} rows to {a.out}", file=sys.stderr)
        return
    if a.stdin:
        text = sys.stdin.buffer.read().decode("utf-8-sig").strip()
        if not text:
            sys.exit("empty stdin")
        print(f"scoring {len(text.split())} words", file=sys.stderr)
        print(f"{score([text])[0]:.4f}")
        return
    if a.files:
        files, base = a.files, None
    elif a.rundir:
        base = os.path.abspath(a.rundir)
        files = sorted(f for f in glob.glob(os.path.join(base, "*", "*", "*.txt"))
                       if not os.path.basename(f).startswith("_system"))
    else:
        ap.error("give a run dir or --files")
    texts = [open(f, encoding="utf-8").read().strip() for f in files]
    ps = score(texts)
    rows = [{"file": os.path.relpath(f, base) if base else f, "words": len(t.split()), "p_ai": round(p, 4)}
            for f, t, p in zip(files, texts, ps)]
    if base:
        with open(os.path.join(base, "detector.jsonl"), "w", encoding="utf-8") as fh:
            for r in rows:
                fh.write(json.dumps(r) + "\n")
        print(f"wrote {len(rows)} rows to {base}/detector.jsonl")
    else:
        for r in rows:
            print(json.dumps(r))


if __name__ == "__main__":
    main()
