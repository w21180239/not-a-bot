#!/usr/bin/env python3
"""Grade a run directory: deterministic lint + LLM judge, then write scores.jsonl and summary.md.

Usage: python3 grade.py runs/<ts> [--workers 6] [--no-judge] [--judge-model sonnet]
"""
import argparse, glob, json, os, re, sys, tempfile
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from statistics import mean
from common import load_cases, call_claude

# ---------- deterministic lint ----------
VOCAB = ["delve", "leverage", "robust", "seamless", "crucial", "pivotal", "tapestry", "utilize",
         "furthermore", "moreover", "it's worth noting", "it is worth noting", "in conclusion",
         "key takeaway"]
RE_NOT_JUST = re.compile(r"\bnot (?:just|only|merely|simply)\b[^.!?\n]{0,80}?\bbut\b", re.I)
RE_ITS_NOT = re.compile(
    r"\b(?:not|isn'?t|aren'?t|wasn'?t|weren'?t)\b[^.!?\n]{1,60}?[,;\u2014\u2013-]\s*(?:it'?s|it is|but|rather|this is|they'?re|that'?s)\b"
    r"|\b(?:isn'?t|is not|aren'?t|are not)\b[^.!?\n]{1,50}[.]\s+(?:It'?s|It is|This is|That'?s)\b", re.I)
RE_US_IZE = re.compile(r"\b(?!(?:size|sized|sizes|sizing|resize|resized|oversize|prize|seize|seized)\b)\w{3,}(?:ize|ized|izes|izing|ization)\b", re.I)
RE_US_OTHER = re.compile(r"\b(?:colors?|colored|behaviors?|favorites?|analyze[sd]?|analyzing)\b", re.I)
RE_BULLET = re.compile(r"^\s*(?:[-*\u2022]|\d+[.)])\s+", re.M)
RE_SPLIT_CONTRAST = re.compile(r"\b(?:does|do|did|is|was) not mean\b[^.!?\n]{1,80}[.!?]\s+(?:it|this|that) (?:means|is)\b", re.I)
RE_STAGED_OTHER = re.compile(r"\b(?:let's dive in|here's the thing|to be clear,|i'm not saying|a tempting approach|at its core|"
                             r"the real question is|let that sink in|that is the real win|the table below)", re.I)
RE_META = re.compile(r"\b(?:here'?s|here is) (?:the|a|your) (?:rewritten|revised|humani[sz]ed|updated)|rewritten version:|\bas an ai\b", re.I)
RE_CJK = re.compile(r"[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff]")


def lint(text):
    words = len(text.split())
    low = text.lower()
    return {
        "words": words,
        "em_dash_per_100w": round(100 * text.count("\u2014") / max(words, 1), 2),
        "em_dashes": text.count("\u2014"),
        "not_just_but": len(RE_NOT_JUST.findall(text)),
        "its_not_its": len(RE_ITS_NOT.findall(text)) + len(RE_SPLIT_CONTRAST.findall(text)),
        "dashes_other": len(re.findall(r"\s[\u2013]\s|\s--\s", text)),
        "staged_other": len(RE_STAGED_OTHER.findall(text)),
        "meta_wrapper": len(RE_META.findall(text)),
        "cjk_leak": int(bool(RE_CJK.search(text))),
        "vocab": sum(len(re.findall(r"\b" + re.escape(v) + r"\b", low)) for v in VOCAB),
        "bold": text.count("**") // 2,
        "bullets": len(RE_BULLET.findall(text)),
        "us_spelling": len(RE_US_IZE.findall(text)) + len(RE_US_OTHER.findall(text)),
    }


# ---------- LLM judge ----------
TELL_PATTERNS = ["negative_parallelism", "rule_of_three", "signpost_closer", "hedge_opener", "puffery",
                 "ai_vocab", "em_dash_overuse", "bold_bullets", "sycophancy", "chatbot_artifact",
                 "excess_hedging", "filler_phrase", "uniform_rhythm", "emoji_decoration",
                 "generic_conclusion", "inline_header_list", "copula_avoidance", "other"]

RUBRIC = """You are a strict evaluator of outgoing workplace prose. You are given a writing TASK, the FACTS that must
survive, NOTES describing what good looks like, and one OUTPUT produced by a writer. You do not know how the
output was produced. Judge only the OUTPUT.

Return ONLY a JSON object, no prose, no code fence, with exactly these keys:
{
 "facts_preserved": [{"fact": "<fact text copied from FACTS>", "status": "present|missing|altered"}],
 "invented_content": {"invented": true|false, "quote": "<quoted invented claim or empty string>"},
 "ai_tells": [{"span": "<exact quoted span from OUTPUT>", "pattern": "<one of: """ + ", ".join(TELL_PATTERNS) + """>"}],
 "reads_human": <integer 1-5>,
 "profile_fit": <integer 1-5>,
 "over_corrected": {"over_corrected": true|false, "quote": "<quoted evidence or empty string>"}
}

Rules:
- facts_preserved: one entry per fact in FACTS, in order. "present" = same meaning, numbers/names/paths/IDs exact (rewording is fine).
  "altered" = changed number/name/meaning. "missing" = absent.
- invented_content: true only for NEW factual claims, commitments, numbers, names or offers not in the TASK or FACTS
  (e.g. an invented deadline, promise, or "happy to hop on a call" offer counts; a polite greeting does not).
  The TASK is the whole supplied material: the draft, earlier messages, pasted tickets, logs or files it contains. A fact
  stated anywhere in the TASK or FACTS is never invented, even when the draft being edited left it out. A fact that
  appears nowhere in the TASK or FACTS is invented even if it is true in the world (a library's release date, a
  vendor's SLA or typical resolution time, a performance figure).
- ai_tells: list every instance of typical LLM-writing tells actually present in the OUTPUT (including ones carried over from a
  draft the writer was asked to fix). Patterns: negative_parallelism ("not just X, it's Y", staged "not X, but Y" used as a
  rhetorical foil), rule_of_three (reflexive triads of adjectives/nouns/clauses), signpost_closer ("let me know if...",
  "in conclusion", "key takeaway"), hedge_opener ("I hope this finds you well", "In today's..."), puffery (significance
  inflation, promotional words), ai_vocab (delve, leverage, robust, seamless, crucial, pivotal, utilize...), em_dash_overuse
  (2+ em dashes in a short text), bold_bullets (bold-labelled bullets), sycophancy ("Great question!"), chatbot_artifact,
  excess_hedging, filler_phrase, uniform_rhythm (every sentence similar length), emoji_decoration, generic_conclusion,
  inline_header_list, copula_avoidance, other.
  DO NOT flag a genuine, factual contrast ("X, not Y" when the distinction is the actual content), a list of three that is
  a real enumeration of three things, a legitimate numbered runbook or ADR structure, or an intentional joke/fragment.
  Judge function, not surface form.
- reads_human: 1 = obviously machine-written, 5 = indistinguishable from a competent human colleague in this context.
- profile_fit: 1-5 for how well register, length and structure match the profile named in the case (neutral international English unless
  the case says otherwise: plain, understated, no hype; do not penalise either UK or US spelling).
- over_corrected: true if the output has awkward or forced casualness/slang, stripped legitimate structure or a legitimate
  contrast/list, lost a joke or fragment that should have stayed, over-compressed so meaning is unclear, or is longer/more
  elaborate than the case warrants. Quote the evidence. Otherwise false.
Be strict and consistent; do not reward length."""


def judge(case, output, model):
    prompt = (f"PROFILE: {case['profile']}\n\nTASK:\n{case['task']}\n\nFACTS:\n"
              + "\n".join(f"- {f}" for f in case["facts"])
              + f"\n\nNOTES (what good looks like):\n{case['notes']}\n\nOUTPUT:\n<<<\n{output}\n>>>")
    for _ in range(3):
        raw = call_claude(prompt, JUDGE_FILE, model, "medium", retries=0)
        try:
            s, e = raw.index("{"), raw.rindex("}")
            return json.loads(raw[s:e + 1])
        except Exception:
            continue
    return {"error": raw[:300]}


def judge_derived(j, facts, context_facts=()):
    if "error" in j:
        return {}
    fp = j.get("facts_preserved", [])
    n_facts = len(facts)
    present = sum(1 for f in fp if f.get("status") == "present")
    ctx = None
    if context_facts:
        idx = [i for i, f in enumerate(facts) if f in set(context_facts)]  # FACTS are judged in order
        ctx = 100 * sum(1 for i in idx if i < len(fp) and fp[i].get("status") == "present") / max(len(idx), 1)
    return {
        "facts_pct": 100 * present / max(n_facts, 1),
        "context_facts_pct": ctx,
        "invented": bool(j.get("invented_content", {}).get("invented")),
        "n_tells": len(j.get("ai_tells", [])),
        "reads_human": j.get("reads_human"),
        "profile_fit": j.get("profile_fit"),
        "over_corrected": bool(j.get("over_corrected", {}).get("over_corrected")),
    }


# ---------- main ----------
def main():
    global JUDGE_FILE
    ap = argparse.ArgumentParser()
    ap.add_argument("rundir")
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--no-judge", action="store_true")
    ap.add_argument("--judge-model", default="sonnet")
    ap.add_argument("--detector-only", action="store_true", help="only append the detector table to summary.md")
    a = ap.parse_args()
    run = os.path.abspath(a.rundir)
    if a.detector_only:
        summarize_detector(run)
        return
    cases = {c["id"]: c for c in load_cases()}
    JUDGE_FILE = os.path.join(tempfile.gettempdir(), "not-a-bot-skill-eval-judge-system.txt")
    open(JUDGE_FILE, "w", encoding="utf-8").write(RUBRIC)

    sp = os.path.join(run, "scores.jsonl")
    prev = {}
    if os.path.exists(sp):
        for l in open(sp, encoding="utf-8"):
            r = json.loads(l)
            if "error" not in r.get("judge", {"error": 1}):
                prev[r["file"]] = r

    files = sorted(glob.glob(os.path.join(run, "*", "*", "*.txt")))
    files = [f for f in files if not os.path.basename(f).startswith("_system")]

    def one(f):
        rel = os.path.relpath(f, run)
        if rel in prev:
            return prev[rel]
        variant, model, name = rel.split(os.sep)
        m = re.match(r"(.+)-(\d+)\.txt$", name)
        cid, rep = m.group(1), int(m.group(2))
        text = open(f, encoding="utf-8").read().strip()
        rec = {"file": rel, "variant": variant, "model": model, "case": cid, "rep": rep,
               "profile": cases[cid]["profile"], "lint": lint(text)}
        if text.startswith("ERROR"):
            rec["judge"] = {"error": text}
        elif not a.no_judge:
            rec["judge"] = judge(cases[cid], text, a.judge_model)
            rec["derived"] = judge_derived(rec["judge"], cases[cid]["facts"], cases[cid].get("context_facts", ()))
        return rec

    with ThreadPoolExecutor(a.workers) as ex:
        recs = list(ex.map(one, files))
    with open(sp, "w", encoding="utf-8") as f:
        for r in recs:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    write_summary(run, recs)
    summarize_detector(run)
    print(os.path.join(run, "summary.md"))


BUCKETS = [("<50", 0, 50), ("50-150", 50, 151), (">150", 151, 10**9)]


def summarize_detector(run):
    """Append detector table (mean p_ai / % >0.5 by variant x model x word bucket) to summary.md.
    Human control: <run>/detector-human.jsonl (same format) if present. Dev-time, noisy signal only."""
    dp = os.path.join(run, "detector.jsonl")
    if not os.path.exists(dp):
        return
    rows = [json.loads(l) for l in open(dp, encoding="utf-8") if l.strip()]
    groups = defaultdict(list)
    for r in rows:
        v, m = r["file"].split(os.sep)[:2]
        groups[(v, m)].append(r)
    hp = os.path.join(run, "detector-human.jsonl")
    if os.path.exists(hp):
        groups[("human-control", "-")] = [json.loads(l) for l in open(hp, encoding="utf-8") if l.strip()]

    def cell(g):
        return "-" if not g else f"{mean(x['p_ai'] for x in g):.2f} / {100*sum(x['p_ai']>0.5 for x in g)/len(g):.0f}% (n={len(g)})"

    L = ["", "## Detector (dev-time, noisy; mean p_ai / % above 0.5)", "",
         "| variant | model | all | " + " | ".join(f"{b[0]} words" for b in BUCKETS) + " |",
         "|---|---|---|" + "---|" * len(BUCKETS)]
    for k in sorted(groups):
        g = groups[k]
        L.append(f"| {k[0]} | {k[1]} | {cell(g)} | " +
                 " | ".join(cell([x for x in g if lo <= x["words"] < hi]) for _, lo, hi in BUCKETS) + " |")
    sm = os.path.join(run, "summary.md")
    txt = open(sm, encoding="utf-8").read() if os.path.exists(sm) else ""
    txt = txt.split("\n## Detector (dev-time")[0].rstrip("\n") + "\n" + "\n".join(L) + "\n"
    open(sm, "w", encoding="utf-8").write(txt)


def fmt(x, d=2):
    return "-" if x is None else f"{x:.{d}f}"


def write_summary(run, recs):
    ok = [r for r in recs if r.get("derived")]
    groups = defaultdict(list)
    for r in ok:
        groups[(r["variant"], r["model"])].append(r)
    L = [f"# Eval summary: {os.path.basename(run)}", "",
         f"{len(recs)} outputs, {len(ok)} judged (judge errors: {sum(1 for r in recs if 'error' in r.get('judge', {}))}). "
         "Small N: treat differences under ~0.3 on 1-5 scales or ~10 points on rates as noise.", "",
         "## Judge metrics by variant x model", "",
         "| variant | model | n | facts % | context facts % | invented | tells/output | reads_human | profile_fit | over_corrected | words |",
         "|---|---|---|---|---|---|---|---|---|---|---|"]
    for k in sorted(groups):
        g = groups[k]
        d = lambda key: [x["derived"][key] for x in g if x["derived"].get(key) is not None]
        L.append(f"| {k[0]} | {k[1]} | {len(g)} | {fmt(mean(d('facts_pct')),1)} | "
                 f"{fmt(mean(d('context_facts_pct')) if d('context_facts_pct') else None,1)} | {fmt(100*mean(map(float,d('invented'))),0)}% | "
                 f"{fmt(mean(d('n_tells')))} | {fmt(mean(d('reads_human')))} | {fmt(mean(d('profile_fit')))} | "
                 f"{fmt(100*mean(map(float,d('over_corrected'))),0)}% | {fmt(mean(x['lint']['words'] for x in g),0)} |")

    L += ["", "## Deterministic lint (mean per output)", "",
          "| variant | model | em-dash/100w | other dashes | not-just-but | its-not-its | staged other | vocab | bold | bullets | US spelling | meta wrapper | CJK leak |",
          "|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for k in sorted(groups):
        g = groups[k]
        m = lambda key: fmt(mean(x["lint"].get(key, 0) for x in g))
        L.append(f"| {k[0]} | {k[1]} | {m('em_dash_per_100w')} | {m('dashes_other')} | {m('not_just_but')} | {m('its_not_its')} | "
                 f"{m('staged_other')} | {m('vocab')} | {m('bold')} | {m('bullets')} | {m('us_spelling')} | {m('meta_wrapper')} | {m('cjk_leak')} |")

    # adversarial subset
    adv = defaultdict(list)
    for r in ok:
        if r["case"].startswith("adv-"):
            adv[(r["variant"], r["model"])].append(r)
    L += ["", "## Adversarial (over-flag) cases only", "",
          "| variant | model | n | over_corrected | tells/output | facts % | words |", "|---|---|---|---|---|---|---|"]
    for k in sorted(adv):
        g = adv[k]
        L.append(f"| {k[0]} | {k[1]} | {len(g)} | {fmt(100*mean(float(x['derived']['over_corrected']) for x in g),0)}% | "
                 f"{fmt(mean(x['derived']['n_tells'] for x in g))} | {fmt(mean(x['derived']['facts_pct'] for x in g),1)} | {fmt(mean(x['lint']['words'] for x in g),0)} |")

    # per-case over-correction counts for adversarial cases
    L += ["", "### Over-corrected by adversarial case (count of reps flagged / total)", "",
          "| case | " + " | ".join(f"{v}/{m}" for v, m in sorted(adv)) + " |", "|---|" + "---|" * len(adv)]
    for cid in sorted({r["case"] for k in adv for r in adv[k]}):
        row = []
        for k in sorted(adv):
            g = [x for x in adv[k] if x["case"] == cid]
            row.append(f"{sum(x['derived']['over_corrected'] for x in g)}/{len(g)}")
        L.append(f"| {cid} | " + " | ".join(row) + " |")

    # per-pattern tell frequency
    # Exclude adversarial cases here: tells flagged there are mostly false positives of interest on their own.
    pats = Counter()
    tc = defaultdict(Counter)
    for r in ok:
        for t in r["judge"].get("ai_tells", []):
            p = t.get("pattern", "other")
            pats[p] += 1
            tc[(r["variant"], r["model"])][p] += 1
    keys = sorted(groups)
    L += ["", "## Per-pattern tell frequency (judge-flagged instances, all cases)", "",
          "| pattern | " + " | ".join(f"{v}/{m} (n={len(groups[(v, m)])})" for v, m in keys) + " |",
          "|---|" + "---|" * len(keys)]
    for p, _ in pats.most_common():
        L.append(f"| {p} | " + " | ".join(str(tc[k][p]) for k in keys) + " |")

    # tell frequency among outputs (share of outputs with >=1)
    L += ["", "## Share of outputs with at least one tell of each pattern", "",
          "| pattern | " + " | ".join(f"{v}/{m}" for v, m in keys) + " |", "|---|" + "---|" * len(keys)]
    for p, _ in pats.most_common():
        row = []
        for k in keys:
            g = groups[k]
            row.append(f"{100*sum(any(t.get('pattern')==p for t in x['judge'].get('ai_tells', [])) for x in g)/len(g):.0f}%")
        L.append(f"| {p} | " + " | ".join(row) + " |")

    # examples of over-correction
    L += ["", "## Over-correction examples (judge quotes)", ""]
    n = 0
    for r in ok:
        oc = r["judge"].get("over_corrected", {})
        if oc.get("over_corrected") and n < 12:
            L.append(f"- `{r['file']}`: {oc.get('quote','')[:200]}")
            n += 1
    open(os.path.join(run, "summary.md"), "w", encoding="utf-8").write("\n".join(L) + "\n")


if __name__ == "__main__":
    main()
