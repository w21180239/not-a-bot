"""Prompt lab for not-a-bot rewrite presets.

    python lab.py run [--variants v0-current,v2-human-features] [--limit N] [--results DIR]
    python lab.py summary [--results DIR]

Every LLM output is cached in cache/llm (keyed by model + prompt), detector scores in
cache/detector.jsonl, so reruns resume and only new work is paid for.
"""
import argparse, hashlib, json, os, re, statistics, subprocess, sys, tempfile, time
from concurrent.futures import ThreadPoolExecutor

import llm

HERE = os.path.dirname(os.path.abspath(__file__))
DETECT_PY = os.path.join(HERE, "..", "detector", ".venv", "Scripts", "python.exe")
DETECT = os.path.join(HERE, "..", "detector", "detect.py")
DET_CACHE = os.path.join(HERE, "cache", "detector.jsonl")
LABEL = {"en": "English (en)", "zh": "Chinese (Simplified) (zh)", "ja": "Japanese (ja)"}
WORKERS = 4
TX_BIN = os.path.join(HERE, "..", "..", "target", "release", "not-a-bot.exe")  # build under test

# ---- prompt pieces: copied verbatim from src/providers/cli_llm.rs build_prompt ----
HISTORY = ("The previous step turned this input:\n<<<\n{prev_in}\n>>>\ninto:\n<<<\n{prev_out}\n>>>\n"
           "Keep its choices; do not undo them.\n\n")
ORIGINAL = ("The text you receive may have passed through machine translation and drifted from the original. "
            "Original (in {orig_lang}):\n<<<\n{original}\n>>>\n"
            "First compare the text with the original. Wherever meaning was lost, changed or added (names spelled "
            "differently, numbers, dates, currencies, negations, qualifiers, who did what to whom, a dropped clause), "
            "fix it with the smallest possible change. Do not copy the original's phrasing or structure back in; only "
            "repair meaning. Then do the task below on the repaired text.\n\n")
TRANSLATE = ("Translate the text below from {source} into {target}. Keep the meaning, tone and formatting; keep every "
             "name, number, URL, code span, placeholder and markup exactly as written. Output only the {target} "
             "translation, with no preamble or notes.")
LEAD = ("Translate the text below from {source} into {target}, and while translating rewrite it the way a capable "
        "person writes to one reader, not the way an AI writes. ")
LEAD_SAME = ("Rewrite the text below in {target} the way a capable person writes to one reader, not the way an AI "
             "writes. ")
SHAPE = ("Fix the shape, not just the words: state claims directly instead of 'not just X, it's Y' or 'X, not Y' foils "
         "nobody raised; drop filler openers ('In today's...', 'When it comes to', 'It's worth noting'), signpost "
         "closers and summary lines, run-ups ('Here's the thing'), deep-sounding sayings ('At its core'), inflated "
         "significance and promotional adjectives, and reflexive lists of three. Use plain words and varied sentence "
         "length. ")
HARD = ("Hard rules: keep every fact, name, number, date, currency, ID, path, URL and code span exactly as written "
        "(copy names letter for letter, do not translate or respell them); keep placeholders and markup untouched; "
        "add nothing that is not in the text; keep the meaning and certainty. Output only the rewritten {target} "
        "text, with no preamble or notes.")
# Added in v2+: features of human workplace writing, before the hard rules.
FEATURES = ("Make it read like human workplace writing: use contractions; mix short and long sentences, with a "
            "fragment or two where that is natural; prefer concrete verbs and use fewer adjectives; no semicolons "
            "or em dashes; link ideas with plain connectors (and, but, so); start with the point. ")
TAIL = "\n\nText:\n<<<\n{text}\n>>>\n"
# Round 2: delete content-free sentences (the main tell left in high-scoring outputs).
CUT = ("Delete every sentence that carries no fact for the reader: thanks for patience or hard work, excitement or "
       "pride, commitments and pledges ('we remain committed', 'we will keep improving'), generic lessons or "
       "takeaways, claims of impact with no number ('makes a real difference'), offers to help and sign-off "
       "pleasantries. Keep every sentence that holds a name, number, date, action, decision, instruction or "
       "caveat, and keep the greeting and signature of a letter. ")

T_HUM = LEAD + SHAPE + HARD + TAIL                       # hop 1 of the standard preset
T_HUM_FINAL = HISTORY + ORIGINAL + LEAD + SHAPE + HARD + TAIL    # hop 2: history + original repair
T_HUM_FINAL_V2 = HISTORY + ORIGINAL + LEAD + SHAPE + FEATURES + HARD + TAIL
T_HUM_FINAL_V2_NOHIST = ORIGINAL + LEAD + SHAPE + FEATURES + HARD + TAIL
T_DIRECT = LEAD_SAME + SHAPE + HARD + TAIL
T_PLAIN = TRANSLATE + TAIL
# Round 3: cutting dropped names/places inside fluff sentences ("across New Zealand"); fold them into kept ones.
CUT_FOLD = ("Delete every sentence that carries no fact for the reader: thanks for patience or hard work, excitement "
            "or pride, commitments and pledges ('we remain committed', 'we will keep improving'), generic lessons or "
            "takeaways, claims of impact with no number ('makes a real difference'), offers to help and sign-off "
            "pleasantries. Before deleting one, move any name, place, number, date or amount in it into a sentence "
            "you keep. Keep every sentence that holds an action, decision, instruction or caveat, and keep the "
            "greeting and signature of a letter. ")
CUT_LIGHT = ("Delete sentences that only express feelings or pledges (thanks for patience or hard work, excitement, "
             "pride, 'we remain committed', 'we look forward') and closing pleasantries. Before deleting one, move "
             "any name, place, number, date or amount in it into a sentence you keep. ")
T_HUM_FOLD = LEAD + SHAPE + CUT_FOLD + HARD + TAIL
T_HUM_FINAL_FOLD = HISTORY + ORIGINAL + LEAD + SHAPE + CUT_FOLD + HARD + TAIL
T_HUM_LIGHT = LEAD + SHAPE + CUT_LIGHT + HARD + TAIL
T_HUM_FINAL_LIGHT = HISTORY + ORIGINAL + LEAD + SHAPE + CUT_LIGHT + HARD + TAIL
# Round 5: a repair hop that restores facts only (the meaning-repair `fix` hop put deleted sentences back).
FACTFIX = ("Below are an ORIGINAL text and a REWRITE of it. The rewrite deliberately deleted sentences that only "
           "express feelings, pledges or pleasantries; that is correct, do not restore them. Check only hard facts: "
           "every name, place, product, number, date, amount, currency, ID, version, path and code span in the "
           "original must appear in the rewrite exactly as written. Where one is missing or altered, put it back "
           "with the smallest edit, inside an existing sentence where possible. Change nothing else. Output only the "
           "full rewrite, with no preamble or notes.\n\nORIGINAL:\n<<<\n{original}\n>>>\nREWRITE:\n<<<\n{text}\n>>>\n")
T_HUM_CUT = LEAD + SHAPE + CUT + HARD + TAIL
T_HUM_FINAL_CUT = HISTORY + ORIGINAL + LEAD + SHAPE + CUT + HARD + TAIL
T_DIRECT_CUT = LEAD_SAME + SHAPE + CUT + HARD + TAIL


OC = "opencode-go/deepseek-v4.1-flash"


def hop(model, frm, to, tpl):
    return {"model": model, "from": frm, "to": to, "prompt_template": tpl}


VARIANTS = {
    # not-a-bot claude-cli defaults to haiku (cli_llm.rs); sonnet used here as briefed.
    "v0-current": [hop("sonnet", "en", "zh", T_HUM), hop("sonnet", "zh", "en", T_HUM_FINAL)],
    "v1-direct": [hop("sonnet", "en", "en", T_DIRECT)],
    "v2-human-features": [hop("sonnet", "en", "zh", T_HUM), hop("sonnet", "zh", "en", T_HUM_FINAL_V2)],
    # plain translate hop is not a humanize hop, so no history block.
    "v3-literal-then-humanize": [hop("sonnet", "en", "zh", T_PLAIN), hop("sonnet", "zh", "en", T_HUM_FINAL_V2_NOHIST)],
    "v4-haiku-final": [hop("sonnet", "en", "zh", T_HUM), hop("haiku", "zh", "en", T_HUM_FINAL_V2)],
    "v5-ja-pivot": [hop("sonnet", "en", "ja", T_HUM), hop("sonnet", "ja", "en", T_HUM_FINAL_V2)],
    "v6-cut": [hop("sonnet", "en", "zh", T_HUM_CUT), hop("sonnet", "zh", "en", T_HUM_FINAL_CUT)],
    "v7-direct-cut": [hop("sonnet", "en", "en", T_DIRECT_CUT)],
    "v8-cut-final-only": [hop("sonnet", "en", "zh", T_HUM), hop("sonnet", "zh", "en", T_HUM_FINAL_CUT)],
    # not-a-bot production default (rewrite_llm = opencode-cli).
    "v0-oc": [hop(OC, "en", "zh", T_HUM), hop(OC, "zh", "en", T_HUM_FINAL)],
    "v6-oc": [hop(OC, "en", "zh", T_HUM_CUT), hop(OC, "zh", "en", T_HUM_FINAL_CUT)],
    "v9-oc-fold": [hop(OC, "en", "zh", T_HUM_FOLD), hop(OC, "zh", "en", T_HUM_FINAL_FOLD)],
    "v10-oc-light": [hop(OC, "en", "zh", T_HUM_LIGHT), hop(OC, "zh", "en", T_HUM_FINAL_LIGHT)],
    "v11-oc-factfix": [hop(OC, "en", "zh", T_HUM_LIGHT), hop(OC, "zh", "en", T_HUM_FINAL_LIGHT),
                       hop(OC, "en", "en", FACTFIX)],
    "v14-oc-cut-factfix": [hop(OC, "en", "zh", T_HUM_CUT), hop(OC, "zh", "en", T_HUM_FINAL_CUT),
                           hop(OC, "en", "en", FACTFIX)],
    "v12-oc-3hop": [hop(OC, "en", "zh", T_HUM_LIGHT), hop(OC, "zh", "ja", T_HUM_FINAL_LIGHT),
                    hop(OC, "ja", "en", T_HUM_FINAL_LIGHT)],
    "v13-oc-3hop-factfix": [hop(OC, "en", "zh", T_HUM_LIGHT), hop(OC, "zh", "ja", T_HUM_FINAL_LIGHT),
                            hop(OC, "ja", "en", T_HUM_FINAL_LIGHT), hop(OC, "en", "en", FACTFIX)],
    # The real binary (TX_BIN), so a result transfers to the product as is.
    "tx-standard": [{"chain": "opencode-cli:zh,opencode-cli:en", "model": "tx", "from": "en", "to": "en"}],
    "tx-standard-fix": [{"chain": "opencode-cli:zh,opencode-cli:en,opencode-cli:fix", "model": "tx", "from": "en",
                         "to": "en"}],
}

JUDGE = ("Compare an ORIGINAL text with a REWRITE of it. Reply with ONLY a JSON object, no prose, no code fence:\n"
         '{{"same_meaning": <1-5>, "reads_human": <1-5>, "added_facts": <true|false>}}\n'
         "same_meaning: 5 = every claim, name, number and nuance preserved; 3 = noticeable loss or shift; "
         "1 = meaning changed.\n"
         "reads_human: 5 = reads like a capable person wrote it themselves; 3 = competent but generic; "
         "1 = obvious AI boilerplate.\n"
         "added_facts: true if the rewrite states any fact, claim, name or number that is not in the original.\n\n"
         "ORIGINAL:\n<<<\n{original}\n>>>\nREWRITE:\n<<<\n{output}\n>>>\n")


# ---- pipeline ----
def tx_call(chain, text, rep):
    """Final text of `not-a-bot rewrite --chain`, cached by binary build, chain, text and rep."""
    st = os.stat(TX_BIN)
    k = hashlib.sha256(f"{st.st_size}:{st.st_mtime_ns}\n{chain}\n{rep}\n{text}".encode("utf-8")).hexdigest()[:32]
    path = os.path.join(HERE, "cache", "tx", k + ".json")
    if os.path.exists(path):
        return json.load(open(path, encoding="utf-8"))["output"]
    cmd = [TX_BIN, "rewrite", "--from", "en", "--preserve", "@code", "--format", "json", "--quiet",
           "--timeout-ms", "600000", "--chain", chain]
    for i in range(3):
        p = subprocess.run(cmd, input=text.encode("utf-8"), capture_output=True, timeout=1200)
        try:
            out = json.loads(p.stdout.decode("utf-8").strip().splitlines()[-1])
        except (ValueError, IndexError):
            out = {"error": p.stderr.decode("utf-8", "replace")[:200]}
        if "text" in out:
            os.makedirs(os.path.dirname(path), exist_ok=True)
            json.dump({"chain": chain, "output": out["text"], "missingFacts": out.get("missingFacts", [])},
                      open(path, "w", encoding="utf-8"), ensure_ascii=False)
            return out["text"]
        time.sleep(3 + 5 * i)
    raise RuntimeError(f"not-a-bot: {str(out.get('error'))[:200]}")


def run_hops(hops, row, rep=0):
    """Return list of hop outputs (None on failure)."""
    text, outs = row["text"], []
    orig_lang = LABEL[hops[0]["from"]]
    for h in hops:
        if h.get("chain"):
            try:
                outs.append(tx_call(h["chain"], row["text"], rep))
            except Exception as e:  # noqa: BLE001
                print(f"  fail {row['id']}: {str(e)[:120]}", file=sys.stderr)
                return outs + [None]
            continue
        prompt = h["prompt_template"].format(
            text=text, original=row["text"], source=LABEL[h["from"]], target=LABEL[h["to"]],
            orig_lang=orig_lang, prev_in=outs[-2] if len(outs) > 1 else row["text"] if outs else "",
            prev_out=outs[-1] if outs else "")
        try:
            text = llm.call(h["model"], prompt, rep=rep)
        except Exception as e:  # noqa: BLE001
            print(f"  fail {row['id']}: {str(e)[:120]}", file=sys.stderr)
            return outs + [None]
        outs.append(text)
    return outs


def judge(original, output):
    try:
        raw = llm.call("haiku", JUDGE.format(original=original, output=output), clean=False)
        m = re.search(r"\{.*\}", raw, re.S)
        d = json.loads(m.group(0))
        return {"same_meaning": float(d["same_meaning"]), "reads_human": float(d["reads_human"]),
                "added_facts": bool(d["added_facts"])}
    except Exception as e:  # noqa: BLE001
        print(f"  judge fail: {str(e)[:100]}", file=sys.stderr)
        return None


def sha(t):
    return hashlib.sha256(t.strip().encode("utf-8")).hexdigest()[:32]


def detect(texts):
    """p_ai per text via one detector batch for the texts not yet cached."""
    cache = {}
    if os.path.exists(DET_CACHE):
        for l in open(DET_CACHE, encoding="utf-8"):
            if l.strip():
                d = json.loads(l)
                cache[d["k"]] = d["p_ai"]
    todo = sorted({t for t in texts if sha(t) not in cache}, key=sha)
    if todo:
        print(f"detector: scoring {len(todo)} texts", file=sys.stderr)
        with tempfile.TemporaryDirectory() as d:
            inp, out = os.path.join(d, "in.jsonl"), os.path.join(d, "out.jsonl")
            with open(inp, "w", encoding="utf-8") as f:
                for t in todo:
                    f.write(json.dumps({"text": t}, ensure_ascii=False) + "\n")
            p = subprocess.run([DETECT_PY, DETECT, "--batch", inp, "--out", out], capture_output=True)
            if p.returncode != 0:
                sys.exit("detector failed: " + p.stderr.decode("utf-8", "replace")[-400:])
            rows = [json.loads(l) for l in open(out, encoding="utf-8") if l.strip()]
        os.makedirs(os.path.dirname(DET_CACHE), exist_ok=True)
        with open(DET_CACHE, "a", encoding="utf-8") as f:
            for t, r in zip(todo, rows):
                cache[sha(t)] = r["p_ai"]
                f.write(json.dumps({"k": sha(t), "p_ai": r["p_ai"]}) + "\n")
    return {t: cache[sha(t)] for t in texts}


def words(t):
    return len(t.split())


def cmd_run(a):
    corpus = [json.loads(l) for l in open(os.path.join(HERE, "corpus.jsonl"), encoding="utf-8") if l.strip()]
    if a.limit:
        corpus = [corpus[i * len(corpus) // a.limit] for i in range(a.limit)]
    names = a.variants.split(",") if a.variants else list(VARIANTS)
    for n in names:
        if n not in VARIANTS:
            sys.exit(f"unknown variant {n}; have {', '.join(VARIANTS)}")
    # rep 0 is the original sample; rep k > 0 is stored as "<variant>@r<k>".
    runs = [(n, k, n if k == 0 else f"{n}@r{k}") for n in names for k in range(a.rep)]
    jobs = [(label, r) for _, _, label in runs for r in corpus]
    base = {label: (n, k) for n, k, label in runs}
    print(f"{len(runs)} runs x {len(corpus)} texts; rewriting", file=sys.stderr)
    with ThreadPoolExecutor(WORKERS) as ex:
        hop_outs = list(ex.map(lambda j: run_hops(VARIANTS[base[j[0]][0]], j[1], base[j[0]][1]), jobs))
    finals = {(n, r["id"]): o[-1] if len(o) == len(VARIANTS[base[n][0]]) else None
              for (n, r), o in zip(jobs, hop_outs)}
    texts = [r["text"] for r in corpus] + [v for v in finals.values() if v]
    p_ai = detect(texts)
    print("judging", file=sys.stderr)
    jlist = [(k, v) for k, v in finals.items() if v]
    src = {r["id"]: r for r in corpus}
    with ThreadPoolExecutor(WORKERS) as ex:
        judged = dict(zip([k for k, _ in jlist], ex.map(lambda kv: judge(src[kv[0][1]]["text"], kv[1]), jlist)))
    os.makedirs(a.results, exist_ok=True)
    for n in base:
        with open(os.path.join(a.results, n + ".jsonl"), "w", encoding="utf-8") as f:
            for (jn, r), outs in zip(jobs, hop_outs):
                if jn != n:
                    continue
                out = finals[(n, r["id"])]
                lost = [t for t in r["facts"] if out is None or t not in out]
                f.write(json.dumps({
                    "id": r["id"], "kind": r["kind"], "source": r["text"], "hops": outs, "output": out,
                    "src_p_ai": p_ai[r["text"]], "p_ai": p_ai[out] if out else None,
                    "facts": r["facts"], "facts_lost": lost if out else None,
                    "len_ratio": round(words(out) / words(r["text"]), 3) if out else None,
                    "judge": judged.get((n, r["id"])), "error": out is None,
                    "hop_models": [h.get("chain") or h["model"] + ":" + h["from"] + ">" + h["to"]
                                   for h in VARIANTS[base[n][0]]],
                }, ensure_ascii=False) + "\n")
    cmd_summary(a)


def mean(xs):
    return statistics.fmean(xs) if xs else float("nan")


def cmd_summary(a):
    rows = []
    for fn in sorted(os.listdir(a.results)):
        if not fn.endswith(".jsonl"):
            continue
        name = fn[:-6]
        data = [json.loads(l) for l in open(os.path.join(a.results, fn), encoding="utf-8") if l.strip()]
        ok = [d for d in data if not d["error"]]
        ps = [d["p_ai"] for d in ok]
        js = [d["judge"] for d in ok if d["judge"]]
        total = sum(len(d["facts"]) for d in ok)
        lost = sum(len(d["facts_lost"]) for d in ok)
        rows.append({
            "name": name, "n": len(data), "err": len(data) - len(ok),
            "mean": mean(ps), "median": statistics.median(ps) if ps else float("nan"),
            "lt05": mean([p < 0.5 for p in ps]), "src_mean": mean([d["src_p_ai"] for d in ok]),
            "texts_lost": sum(1 for d in ok if d["facts_lost"]), "tok_ret": 1 - lost / total if total else float("nan"),
            "len": mean([d["len_ratio"] for d in ok]),
            "sm": mean([j["same_meaning"] for j in js]), "rh": mean([j["reads_human"] for j in js]),
            "added": sum(j["added_facts"] for j in js), "judged": len(js),
            "hops": ", ".join(data[0]["hop_models"]) if data else "", "data": ok})
    rows.sort(key=lambda r: r["mean"])
    L = ["# Prompt lab summary", "",
         "p_ai = desklib/ai-text-detector-v1.01 (higher = more AI-like). Facts: exact, case-sensitive token match. "
         "Judge: haiku, 1-5. `fact loss` = texts missing at least one fact token / texts.", "",
         "| variant | n | mean p_ai | median | <0.5 | src mean | fact loss | token ret. | len ratio | same_meaning | reads_human | added_facts | err |",
         "|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        L.append(f"| {r['name']} | {r['n']} | {r['mean']:.3f} | {r['median']:.3f} | {r['lt05']:.0%} | {r['src_mean']:.3f} | "
                 f"{r['texts_lost']}/{r['n'] - r['err']} | {r['tok_ret']:.1%} | {r['len']:.2f} | {r['sm']:.2f} | "
                 f"{r['rh']:.2f} | {r['added']}/{r['judged']} | {r['err']} |")
    pooled = {}
    for r in rows:
        pooled.setdefault(r["name"].split("@")[0], []).append(r)
    multi = {k: v for k, v in pooled.items() if len(v) > 1}
    if multi:
        L += ["", "## Pooled over samples", "",
              "Mean over samples; range = min-max of the per-sample means.", "",
              "| variant | samples | mean p_ai (range) | <0.5 | fact loss (range) | same_meaning | reads_human |",
              "|---|---|---|---|---|---|---|"]
        for k, v in sorted(multi.items(), key=lambda kv: mean([r["mean"] for r in kv[1]])):
            ms, fl = [r["mean"] for r in v], [r["texts_lost"] for r in v]
            L.append(f"| {k} | {len(v)} | {mean(ms):.3f} ({min(ms):.3f}-{max(ms):.3f}) | "
                     f"{mean([r['lt05'] for r in v]):.0%} | {mean(fl):.1f} ({min(fl)}-{max(fl)}) / {v[0]['n']} | "
                     f"{mean([r['sm'] for r in v]):.2f} | {mean([r['rh'] for r in v]):.2f} |")
    L += ["", "## Hops", ""] + [f"- {r['name']}: {r['hops']}" for r in rows]
    L += ["", "## Fact losses", ""]
    for r in rows:
        bad = [d for d in r["data"] if d["facts_lost"]]
        L.append(f"- {r['name']}: " + ("none" if not bad else "; ".join(
            f"{d['id']} [{', '.join(d['facts_lost'])}]" for d in bad)))
    with open(os.path.join(a.results, "summary.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(L) + "\n")
    print(f"wrote {os.path.join(a.results, 'summary.md')}", file=sys.stderr)


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("--variants", help="comma list; default all")
    r.add_argument("--limit", type=int, help="N texts spread evenly over the corpus (smoke)")
    r.add_argument("--rep", type=int, default=1, help="samples per variant; extra ones are <variant>@r<k>")
    s = sub.add_parser("summary")
    for p in (r, s):
        p.add_argument("--results", default=os.path.join(HERE, "results"))
    a = ap.parse_args()
    {"run": cmd_run, "summary": cmd_summary}[a.cmd](a)


if __name__ == "__main__":
    main()
