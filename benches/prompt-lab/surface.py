#!/usr/bin/env python3
"""Spike: deterministic (no-LLM) surface rewrites, ablated per rule.

  python3 surface.py gen [--limit N] --out rows.jsonl     # one row per (doc, variant)
  .venv/bin/python ../detector/detect.py --batch rows.jsonl --out scored.jsonl
  python3 surface.py report scored.jsonl                  # mean p_ai, <0.5 count, facts kept
"""
import argparse, json, os, re, statistics, sys

HERE = os.path.dirname(os.path.abspath(__file__))
LINT_DATA = os.path.join(HERE, "..", "..", "src", "core", "lint_data.rs")


def load_vocab():
    src = open(LINT_DATA, encoding="utf-8").read()
    block = src.split("pub(super) const VOCAB", 1)[1].split("];", 1)[0]
    out = []
    for word, repl in re.findall(r'\("([^"]+)", "([^"]*)"\)', block):
        repl = repl.split(" / ")[0]
        # advice strings, not substitutes
        if repl in ("be specific", "name what it covers", "how X affects Y", "key", "important", "needed"):
            continue
        out.append((word, repl))
    return sorted(out, key=lambda p: -len(p[0]))


VOCAB = load_vocab()


def match_case(src, repl):
    return repl[:1].upper() + repl[1:] if src[:1].isupper() and repl else repl


def r_markdown(t):
    t = re.sub(r"\*\*(.+?)\*\*|__(.+?)__", lambda m: m.group(1) or m.group(2), t)
    t = re.sub(r"(?m)^#{1,6}\s+", "", t)
    t = re.sub(r"(?m)^\s*[-*\u2022]\s+", "", t)
    t = re.sub(r"(?m)^([^:\n]{1,30}):\s+(?=[A-Z])", lambda m: m.group(1) + ": ", t)
    return t


def r_dash(t):
    t = re.sub(r"\s*[\u2014]\s*|\s+[\u2013]\s+|\s+--\s+", ", ", t)
    return t


def r_vocab(t):
    for word, repl in VOCAB:
        pat = re.compile(r"\b" + re.escape(word) + r"\b", re.I)
        if repl:
            t = pat.sub(lambda m: match_case(m.group(0), repl), t)
        else:
            t = pat.sub("", t)
    t = re.sub(r"[ \t]{2,}", " ", t)
    t = re.sub(r" ([,.])", r"\1", t)
    t = re.sub(r"(^|[.!?]\s+|\n)([a-z])", lambda m: m.group(1) + m.group(2).upper(), t)
    return t


FILLER = re.compile(
    r"don't hesitate|do not hesitate|happy to help|thank you for reaching out|thanks for reaching out"
    r"|we truly appreciate|appreciate your patience|hope this helps|let me know if you have any questions"
    r"|valued customer|for any inconvenience|we understand how important|in summary|in conclusion|overall,"
    r"|feel free to|please let me know|i hope you|hope you're well|hope you are well|looking forward to",
    re.I,
)
SENT = re.compile(r"(?:[^.!?\n]|\.(?=\w))+[.!?]+[\"')\]]*|(?:[^.!?\n]|\.(?=\w))+$")


def r_filler(t):
    out = []
    for line in t.split("\n"):
        kept = [s for s in SENT.findall(line) if not (FILLER.search(s) and not re.search(r"\d|\S\s+[A-Z][a-z]", s))]
        if line.strip() and not kept:
            continue  # whole line was filler
        out.append(" ".join(s.strip() for s in kept) if kept else line)
    return re.sub(r"\n{3,}", "\n\n", "\n".join(out)).strip()


INTENS = re.compile(r"\b(truly|really|very|sincerely|incredibly|extremely|highly|deeply|greatly|absolutely|genuinely)\s+", re.I)


def r_intens(t):
    return INTENS.sub("", t)


CONTR = [
    (r"\bwe are\b", "we're"), (r"\bit is\b", "it's"), (r"\bI am\b", "I'm"), (r"\bdo not\b", "don't"),
    (r"\bdoes not\b", "doesn't"), (r"\bdid not\b", "didn't"), (r"\bis not\b", "isn't"), (r"\bare not\b", "aren't"),
    (r"\bwill not\b", "won't"), (r"\bcannot\b", "can't"), (r"\bwe will\b", "we'll"), (r"\bI will\b", "I'll"),
    (r"\byou will\b", "you'll"), (r"\bthat is\b", "that's"), (r"\bwe have\b", "we've"), (r"\bthere is\b", "there's"),
    (r"\byou are\b", "you're"), (r"\bthey are\b", "they're"), (r"\bhave not\b", "haven't"), (r"\bwas not\b", "wasn't"),
]


def r_contract(t):
    for pat, rep in CONTR:
        t = re.sub(pat, lambda m: match_case(m.group(0), rep), t)
        t = re.sub(pat, lambda m: match_case(m.group(0), rep), t, flags=re.I)
    return t


def r_split(t):
    # split long sentences at "; " or ", and " / ", which "
    def split_one(s):
        if len(s.split()) < 24:
            return s
        s2 = re.sub(r";\s+([a-z])", lambda m: ". " + m.group(1).upper(), s, count=1)
        if s2 == s:
            s2 = re.sub(r",\s+and\s+([a-z])", lambda m: ". " + m.group(1).upper(), s, count=1)
        return s2
    return "\n".join(" ".join(split_one(x) for x in SENT.findall(line)) if line.strip() else line for line in t.split("\n"))


RULES = {
    "md": r_markdown, "dash": r_dash, "vocab": r_vocab, "filler": r_filler,
    "intens": r_intens, "contract": r_contract, "split": r_split,
}
ORDER = ["md", "filler", "vocab", "intens", "dash", "contract", "split"]


def apply(t, names):
    for n in ORDER:
        if n in names:
            t = RULES[n](t)
    return t


def gen(a):
    docs = [json.loads(l) for l in open(os.path.join(HERE, "corpus.jsonl"), encoding="utf-8") if l.strip()]
    if a.limit:
        docs = docs[: a.limit]
    variants = {"source": []}
    variants.update({r: [r] for r in ORDER})
    variants["all"] = ORDER
    variants["all-no-split"] = [r for r in ORDER if r != "split"]
    with open(a.out, "w", encoding="utf-8") as fh:
        for d in docs:
            for v, names in variants.items():
                fh.write(json.dumps({"id": d["id"], "variant": v, "facts": d["facts"],
                                     "text": apply(d["text"], names)}, ensure_ascii=False) + "\n")
    print(f"wrote {len(docs) * len(variants)} rows to {a.out}")


def report(a):
    rows = [json.loads(l) for l in open(a.scored, encoding="utf-8") if l.strip()]
    by = {}
    for r in rows:
        by.setdefault(r["variant"], []).append(r)
    src = {r["id"]: r for r in by.get("source", [])}
    print(f"{'variant':14} {'n':>3} {'mean_p':>7} {'median':>7} {'<0.5':>5} {'facts_lost':>10} {'chg_vs_src':>10}")
    for v, rs in by.items():
        ps = [r["p_ai"] for r in rs]
        lost = sum(1 for r in rs for f in r["facts"] if f not in r["text"])
        delta = statistics.mean(r["p_ai"] - src[r["id"]]["p_ai"] for r in rs)
        print(f"{v:14} {len(rs):3} {statistics.mean(ps):7.3f} {statistics.median(ps):7.3f} "
              f"{sum(p < 0.5 for p in ps):5} {lost:10} {delta:+10.3f}")


ap = argparse.ArgumentParser()
sub = ap.add_subparsers(dest="cmd", required=True)
g = sub.add_parser("gen"); g.add_argument("--limit", type=int); g.add_argument("--out", required=True)
r = sub.add_parser("report"); r.add_argument("scored")
a = ap.parse_args()
{"gen": gen, "report": report}[a.cmd](a)
