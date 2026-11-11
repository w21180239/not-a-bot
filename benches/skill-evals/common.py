"""Shared helpers for the not-a-bot skill eval harness."""
import json, os, subprocess, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
# the skill under test: <repo>/integrations/skill/not-a-bot
SKILL_DIR = os.path.abspath(os.path.join(HERE, "..", "..", "integrations", "skill", "not-a-bot"))
BASE_LINE = ("You write and edit text that people outside this conversation will read. "
             "Output only the final text.")
# --bare needs ANTHROPIC_API_KEY; here auth is via login, so isolate with --setting-sources "".
ISOLATE = ["--setting-sources", ""]
ENV = dict(os.environ, CLAUDE_CODE_DISABLE_CLAUDE_MDS="1", CLAUDE_CODE_DISABLE_AUTO_MEMORY="1")
CWD = tempfile.gettempdir()  # neutral cwd, no project CLAUDE.md


def load_cases(path=None):
    path = path or os.path.join(HERE, "cases.jsonl")
    with open(path, encoding="utf-8") as f:
        return [json.loads(l) for l in f if l.strip()]


def build_system_prompt(skill_dir):
    """BASE_LINE + SKILL.md + profiles/* + references/* + patterns.md (+ dimensions.md if present)."""
    parts = [BASE_LINE]
    files = [os.path.join(skill_dir, "SKILL.md")]
    for sub in ("profiles", "references"):
        d = os.path.join(skill_dir, sub)
        if os.path.isdir(d):
            files += [os.path.join(d, n) for n in sorted(os.listdir(d)) if os.path.isfile(os.path.join(d, n))]
    for extra in ("patterns.md", "dimensions.md"):
        p = os.path.join(skill_dir, extra)
        if os.path.exists(p):
            files.append(p)
    for p in files:
        rel = os.path.relpath(p, skill_dir)
        with open(p, encoding="utf-8") as f:
            parts.append(f"\n\n===== {rel} =====\n{f.read()}")
    return "".join(parts)


def call_claude(prompt, system_file, model, effort, timeout=420, retries=1):
    cmd = ["claude", "-p", *ISOLATE, "--model", model, "--effort", effort,
           "--tools", "", "--system-prompt-file", system_file]
    last = ""
    for _ in range(retries + 1):
        try:
            r = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                               encoding="utf-8", errors="replace",  # Windows locale may not be UTF-8
                               timeout=timeout, env=ENV, cwd=CWD)
            out = r.stdout.strip()
            if r.returncode == 0 and out:
                return out
            last = f"ERROR rc={r.returncode} {r.stderr.strip()[:300]}"
        except subprocess.TimeoutExpired:
            last = "ERROR timeout"
    return last
