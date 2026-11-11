#!/usr/bin/env python3
"""Run cases x variants x models x reps through headless Claude Code.

Usage: python3 run.py [--variants none,current] [--reps 3] [--workers 6]
       [--cases id1,id2] [--current-dir DIR] [--v2-dir DIR] [--dirs name=DIR,...]
       [--out runs/<ts>]
"""
import argparse, os, sys, time
from concurrent.futures import ThreadPoolExecutor, as_completed
from common import HERE, SKILL_DIR, BASE_LINE, build_system_prompt, call_claude, load_cases

MODELS = {"haiku": "high", "opus": "medium"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--variants", default="none,current")
    ap.add_argument("--models", default="haiku,opus")
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--cases", default="")
    ap.add_argument("--current-dir", default=SKILL_DIR)
    ap.add_argument("--v2-dir", default=SKILL_DIR)
    ap.add_argument("--dirs", default="", help="extra variants as name=DIR,name=DIR (name may also override current/v2)")
    ap.add_argument("--out", default="")
    a = ap.parse_args()

    out = a.out or os.path.join(HERE, "runs", time.strftime("%Y%m%d-%H%M%S"))
    os.makedirs(out, exist_ok=True)
    dirs = {"current": a.current_dir, "v2": a.v2_dir}
    for pair in filter(None, a.dirs.split(",")):
        name, sep, path = pair.partition("=")
        if not sep or not name or not path:
            ap.error(f"--dirs expects name=DIR pairs, got {pair!r}")
        dirs[name] = os.path.abspath(path)
    prompts = {}
    for v in a.variants.split(","):
        if v != "none" and v not in dirs:
            ap.error(f"variant {v!r} has no directory; pass --dirs {v}=PATH")
        text = BASE_LINE if v == "none" else build_system_prompt(dirs[v])
        pf = os.path.join(out, f"_system-{v}.txt")
        with open(pf, "w", encoding="utf-8") as f:
            f.write(text)
        prompts[v] = pf
        print(f"variant {v}: {len(text)} chars", file=sys.stderr)

    cases = load_cases()
    if a.cases:
        keep = set(a.cases.split(","))
        cases = [c for c in cases if c["id"] in keep]

    jobs = []
    for v in prompts:
        for m in a.models.split(","):
            for c in cases:
                for r in range(1, a.reps + 1):
                    p = os.path.join(out, v, m, f"{c['id']}-{r}.txt")
                    if not os.path.exists(p):
                        jobs.append((v, m, c, r, p))
    print(f"{len(jobs)} calls -> {out}", file=sys.stderr)

    def work(j):
        v, m, c, r, p = j
        text = call_claude(c["task"], prompts[v], m, MODELS[m])
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w", encoding="utf-8") as f:
            f.write(text + "\n")
        return j, text

    done = 0
    with ThreadPoolExecutor(a.workers) as ex:
        for fut in as_completed([ex.submit(work, j) for j in jobs]):
            j, text = fut.result()
            done += 1
            tag = "ERR" if text.startswith("ERROR") else "ok"
            print(f"[{done}/{len(jobs)}] {j[0]}/{j[1]}/{j[2]['id']}-{j[3]} {tag}", file=sys.stderr)
    print(out)


if __name__ == "__main__":
    main()
