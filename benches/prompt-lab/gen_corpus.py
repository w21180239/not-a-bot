"""Generate corpus.jsonl: 30 AI-style English workplace texts with deterministic fact tokens."""
import json, os, random, re
from concurrent.futures import ThreadPoolExecutor
import llm

HERE = os.path.dirname(os.path.abspath(__file__))
KINDS = {
    "customer_email": "a customer email (support reply, order update, refund or onboarding message)",
    "slack_update": "an internal Slack or status update to a team (project progress, incident follow-up, release note)",
    "marketing_blurb": "a product or marketing blurb (landing page section, feature announcement, app store text)",
    "documentation": "a documentation or how-to paragraph (setup steps, config explanation, troubleshooting note)",
    "linkedin_post": "a LinkedIn-style post (project milestone, lessons learned, hiring or launch announcement)",
}
INDUSTRIES = ["a Wellington fintech", "an Auckland logistics firm", "a Christchurch SaaS startup",
              "a Dunedin health-tech company", "a Hamilton agritech business", "a Queenstown tourism operator",
              "a Tauranga e-commerce shop", "an Auckland council contractor", "a Nelson craft brewery",
              "a remote-first developer-tools company", "a Wellington government agency supplier",
              "an Auckland property management firm"]
FACT_MIX = ["a dollar amount, a date and a person's name", "a ticket or invoice ID, a percentage and two names",
            "an inline `code span`, a version number and a deadline",
            "an email address or URL, a headcount and a place name",
            "a NZD price, a time of day and an order number",
            "a metric with a number, a product name and a named teammate"]
SYSTEM = ("Write {desc} for {industry}. Write it the way a typical AI assistant would: polished, upbeat, "
          "generic corporate phrasing. Include concrete facts: {mix}. Use numerals for numbers and dates, "
          "sentence case, no hashtags, no emojis, no markdown headings or bold. {extra}"
          "Length: 90 to 140 words. Output only the text itself, nothing else.")
EXTRA = {"customer_email": "Include a greeting and a sign-off. ", "slack_update": "", "marketing_blurb": "",
         "documentation": "Use backticks for any command, setting or file name. ", "linkedin_post": ""}

URL = re.compile(r"https?://[^\s<>)\]\"']+")
EMAIL = re.compile(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+")
CODE = re.compile(r"`[^`\n]+`")
ID = re.compile(r"(?<![\w-])(?:[A-Z]{2,}(?:-[A-Z0-9]+)*-\d+|#\d+)(?!\w)")
ALNUM = re.compile(r"(?<![\w.])[A-Za-z]+\d+(?:\.\d+)*(?!\w)")
VER = re.compile(r"(?<![\w.])\d+(?:\.\d+){2,}(?!\w)")
NUM = re.compile(r"(?<![\w])(?:NZ\$|US\$|A\$|[$\u20ac\u00a3])?\d+(?:,\d{3})*(?:\.\d+)?(?::\d{2})?"
                 r"(?:\s?[ap]m\b|%|[kKmM]\b)?")
NAME = re.compile(r"\b[A-Z][^\W\d_]+\b")
STOP = {"Hi", "Hello", "Dear", "Thanks", "Thank", "Cheers", "Regards", "Best", "Kind", "Team", "Support"}


def extract_facts(text):
    """Deterministic must-survive tokens: code spans, URLs, emails, IDs, versions, numbers, mid-sentence names."""
    facts, masked = [], text

    def take(rx, strip=""):
        nonlocal masked
        for m in rx.finditer(masked):
            facts.append(m.group(0).rstrip(strip) if strip else m.group(0))
        masked = rx.sub(lambda m: " " * len(m.group(0)), masked)

    take(CODE); take(URL, ".,;:!?"); take(EMAIL, ".,;:"); take(ID); take(ALNUM); take(VER); take(NUM)
    for m in NAME.finditer(masked):
        w = m.group(0)
        pre = masked[:m.start()].rstrip(" \t\"'“‘*_#-•([")
        if pre == "" or pre[-1] in ".!?:;\n" or w in STOP:
            continue
        facts.append(w)
    seen, out = set(), []
    for f in facts:
        if f not in seen:
            seen.add(f)
            out.append(f)
    return out


def gen_one(kind, idx):
    rng = random.Random(f"{kind}-{idx}")
    for attempt in range(4):
        prompt = SYSTEM.format(desc=KINDS[kind], industry=rng.choice(INDUSTRIES), mix=rng.choice(FACT_MIX),
                               extra=EXTRA[kind])
        if attempt:
            prompt += f" (Variation {attempt}.)"
        text = llm.call("sonnet", prompt, clean=False).strip()
        n = len(text.split())
        facts = extract_facts(text)
        if 70 <= n <= 160 and len(facts) >= 4 and "```" not in text:
            return {"id": f"{kind}-{idx}", "kind": kind, "text": text, "facts": facts}
    raise RuntimeError(f"no valid text for {kind}-{idx}")


def main():
    jobs = [(k, i) for k in KINDS for i in range(1, 7)]
    with ThreadPoolExecutor(4) as ex:
        rows = list(ex.map(lambda j: gen_one(*j), jobs))
    with open(os.path.join(HERE, "corpus.jsonl"), "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    print(f"wrote {len(rows)} rows")


if __name__ == "__main__":
    main()
