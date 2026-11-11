"""Cached `claude -p` calls (prompt on stdin, neutral cwd, no CLAUDE.md or memory)."""
import hashlib, json, os, shutil, subprocess, tempfile, threading, time

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, "cache", "llm")
NEUTRAL_CWD = tempfile.gettempdir()
_locks, _guard = {}, threading.Lock()


def key_of(model, prompt):
    return hashlib.sha256((model + "\n" + prompt).encode("utf-8")).hexdigest()[:32]


def strip_meta_line(text):
    """Port of cli_llm.rs strip_meta_line: drop a leading 'Here is the translation:' line."""
    text = text.strip()
    if "\n" not in text:
        return text
    first, rest = text.split("\n", 1)
    first = first.strip()
    low = first.lower()
    en = ("here is", "here's", "sure", "certainly", "translation:", "rewritten")
    meta = first.endswith((":", "：")) and (
        any(low.startswith(p) for p in en)
        or first.startswith(("以下是", "以下は"))
        or "translation" in low or "翻译" in first or "翻訳" in first)
    return rest.strip() if meta else text


def _run_opencode(model, prompt, timeout):
    """Same invocation as translatex's opencode-cli provider; model like opencode-go/deepseek-v4.1-flash."""
    exe = shutil.which("opencode")
    p = subprocess.run([exe, "run", "--archived", "-m", model, "--format", "json"],
                       input=prompt.encode("utf-8"), capture_output=True, cwd=NEUTRAL_CWD, timeout=timeout)
    text = ""
    for line in p.stdout.decode("utf-8", "replace").splitlines():
        try:
            ev = json.loads(line)
        except ValueError:
            continue
        if ev.get("type") == "text":
            text += ev.get("part", {}).get("text", "")
        elif ev.get("type") == "error":
            raise RuntimeError(f"opencode error: {str(ev.get('error'))[:200]}")
    text = text.strip()
    if p.returncode != 0 or not text:
        raise RuntimeError(f"opencode exit {p.returncode}: {p.stderr.decode('utf-8', 'replace')[:200]}")
    return text


def _run(model, prompt, timeout):
    if model.startswith("opencode-go/"):
        return _run_opencode(model, prompt, timeout)
    exe = shutil.which("claude")
    env = dict(os.environ, CLAUDE_CODE_DISABLE_CLAUDE_MDS="1", CLAUDE_CODE_DISABLE_AUTO_MEMORY="1")
    env.pop("CLAUDECODE", None)
    p = subprocess.run([exe, "-p", "--model", model, "--setting-sources", "", "--tools", ""],
                       input=prompt.encode("utf-8"), capture_output=True, cwd=NEUTRAL_CWD,
                       env=env, timeout=timeout)
    out = p.stdout.decode("utf-8", "replace").strip()
    if p.returncode != 0 or not out:
        raise RuntimeError(f"claude exit {p.returncode}: {p.stderr.decode('utf-8', 'replace')[:200]} {out[:100]}")
    return out


def call(model, prompt, timeout=300, retries=2, clean=True):
    """Return output text; cached on disk by (model, prompt). Raises after retries."""
    os.makedirs(CACHE, exist_ok=True)
    k = key_of(model, prompt)
    path = os.path.join(CACHE, k + ".json")
    with _guard:
        lock = _locks.setdefault(k, threading.Lock())
    with lock:
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                return json.load(f)["output"]
        err = None
        for i in range(retries + 1):
            try:
                out = _run(model, prompt, timeout)
                if clean:
                    out = strip_meta_line(out)
                tmp = path + f".{os.getpid()}.tmp"
                with open(tmp, "w", encoding="utf-8") as f:
                    json.dump({"model": model, "prompt": prompt, "output": out}, f, ensure_ascii=False)
                os.replace(tmp, path)
                return out
            except Exception as e:  # noqa: BLE001
                err = e
                time.sleep(2 + 3 * i)
        raise RuntimeError(str(err))
