#!/usr/bin/env python3
"""Compare round-trip configurations: rewrite strength vs fact loss vs surviving AI tells.

Runs each corpus text through every configuration with `translatex roundtrip` and writes
results.jsonl and summary.md to --out.

Usage: python benches/roundtrip-depth/measure.py [--bin PATH] [--out DIR] [--configs mt,llm] [--sleep 0.4]
"""
import argparse, json, os, re, shutil, subprocess, sys, time
from statistics import mean

HERE = os.path.dirname(os.path.abspath(__file__))

# name -> extra roundtrip arguments
CONFIGS = {
    "mt-d1": ["--depth", "1"],
    "mt": ["--preset", "mt"],  # default: 2 MT hops
    "mt-deep": ["--preset", "mt-deep"],
    "mt-deep-mixed": ["--chain", "google:{piv},edge:ja,google:fi,edge:{src}"],
    "llm-claude": ["--chain", "claude-cli:{piv},claude-cli:{src}"],
    "llm-opencode": ["--chain", "opencode-cli:{piv},opencode-cli:{src}"],
    "lynote-claude": ["--chain", "claude-cli:{piv},claude-cli:ja,google:fi,edge:{src}"],
    "lynote-opencode": ["--chain", "opencode-cli:{piv},opencode-cli:ja,google:fi,edge:{src}"],
    "mt-fix-claude": ["--chain", "{piv},{src},claude-cli:fix"],
    "lynote-fix-claude": ["--chain", "claude-cli:{piv},claude-cli:ja,google:fi,edge:{src},claude-cli:fix"],
    "lynote-fix-opencode": ["--chain", "opencode-cli:{piv},opencode-cli:ja,google:fi,edge:{src},opencode-cli:fix"],
    "mt-then-llm-claude": ["--chain", "edge:{piv},google:ja,claude-cli:{src}"],
    "mt-then-llm-opencode": ["--chain", "edge:{piv},google:ja,opencode-cli:{src}"],
}
PRESERVE = "@code"  # named pattern: survives Windows .cmd shims
STOP = set("a an the of to in on for and or is are was were be been it its it's this that with as at by from "
           "our we us you your their they so until will can not just than more most has have had do does".split())
STOCK = ["fast-paced", "landscape", "important to note", "worth noting", "comprehensive", "leverage",
         "cutting-edge", "empower", "not just", "fundamental shift", "seamless", "at its core", "fostering",
         "robust", "testament", "unwavering", "move the needle", "when it comes to", "navigate",
         "不仅仅", "更是", "赋能", "抓手", "值得注意", "当今"]


def content_words(text, lang):
    if lang == "zh":
        return set(re.findall(r"[一-鿿]", text))
    return {w for w in re.findall(r"[a-z0-9']+", text.lower()) if w not in STOP}


def run(binary, case, args, timeout):
    src = case["lang"]
    piv = "en" if src == "zh" else "zh"
    args = [a.replace("{src}", src).replace("{piv}", piv) for a in args]
    cmd = [binary, "roundtrip", "--from", src, "--preserve", PRESERVE, "--format", "json", "--quiet", *args]
    t0 = time.time()
    try:
        r = subprocess.run(cmd, input=case["text"], capture_output=True, text=True, encoding="utf-8",
                           timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"error": "timeout"}
    wall = int(1000 * (time.time() - t0))
    try:
        out = json.loads(r.stdout.strip().splitlines()[-1])
    except Exception:
        return {"error": f"rc={r.returncode} {r.stderr.strip()[:200]}"}
    if "error" in out:
        return {"error": out["error"].get("code", "ERROR"), "message": out["error"].get("message", "")[:200]}
    return {"text": out["text"], "latency_ms": wall, "missing_facts": out.get("missingFacts", []),
            "hops": [f"{h.get('provider')}:{h.get('to')}" for h in out.get("hops", [])]}


def score(case, res):
    src, out, lang = case["text"], res["text"], case["lang"]
    a, b = content_words(src, lang), content_words(out, lang)
    stock_src = [s for s in STOCK if s in src.lower()]
    return {"overlap": round(len(a & b) / max(len(a | b), 1), 3),
            "len_delta": round(abs(len(out) - len(src)) / len(src), 3),
            "stock_in_source": len(stock_src),
            "stock_kept": [s for s in stock_src if s in out.lower()]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default=shutil.which("translatex") or "translatex")
    ap.add_argument("--out", default=os.path.join(HERE, "out"))
    ap.add_argument("--configs", default=",".join(CONFIGS))
    ap.add_argument("--sleep", type=float, default=0.4)
    ap.add_argument("--timeout", type=int, default=600)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    cases = [json.loads(l) for l in open(os.path.join(HERE, "corpus.jsonl"), encoding="utf-8") if l.strip()]
    names = a.configs.split(",")
    path = os.path.join(a.out, "results.jsonl")
    done = set()
    rows = []
    if os.path.exists(path):  # resume: keep finished rows
        for l in open(path, encoding="utf-8"):
            r = json.loads(l)
            if "text" in r:
                rows.append(r)
                done.add((r["config"], r["id"]))
    with open(path, "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
        for name in names:
            for c in cases:
                if (name, c["id"]) in done:
                    continue
                res = run(a.bin, c, CONFIGS[name], a.timeout)
                row = {"config": name, "id": c["id"], "kind": c["kind"], "lang": c["lang"], **res}
                if "text" in res:
                    row.update(score(c, res))
                rows.append(row)
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
                f.flush()
                time.sleep(a.sleep)
                print(f"{name} {c['id']} {'ERR ' + str(res.get('error')) if 'error' in res else 'ok'}", file=sys.stderr)
    L = ["| config | ok | overlap (lower = more rewrite) | length change | missing facts (guard) | stock phrases kept | latency |",
         "|---|---|---|---|---|---|---|"]
    for name in names:
        g = [r for r in rows if r["config"] == name and "text" in r]
        n = len([r for r in rows if r["config"] == name])
        if not g:
            L.append(f"| {name} | 0/{n} | | | | | |")
            continue
        L.append(f"| {name} | {len(g)}/{n} | {mean(r['overlap'] for r in g):.3f} | "
                 f"{100 * mean(r['len_delta'] for r in g):.1f}% | {sum(len(r['missing_facts']) for r in g)} | "
                 f"{sum(len(r['stock_kept']) for r in g)}/{sum(r['stock_in_source'] for r in g)} | "
                 f"{mean(r['latency_ms'] for r in g) / 1000:.1f} s |")
    open(os.path.join(a.out, "summary.md"), "w", encoding="utf-8").write("\n".join(L) + "\n")
    print("\n".join(L))


if __name__ == "__main__":
    main()
