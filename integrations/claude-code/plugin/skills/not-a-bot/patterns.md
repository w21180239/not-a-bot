# AI writing patterns (audit reference)

Read this during the audit pass, after the draft exists. Families are ordered by how often they showed up in current-model output (Haiku and Opus, October 2026 eval), strongest first. One sighting of a pattern in families 1-4 justifies an edit. Patterns marked *weak alone* need a second tell nearby before you change anything, because people use them too.

Look at paragraph shape as well as sentences. A contrast split across two sentences, three parallel examples, or the same closer after every section is the same tell at a larger scale, and each sentence can read fine on its own.

Contents
1. Decorated structure: bold-labelled bullets, headers on short text, decorative headings and rules, emoji
2. Padding: filler phrases, chatbot artefacts, hedging, knowledge-limit hedges
3. Staging: set-up-and-knock-down contrasts, run-ups, arguing with no one, deep-sounding sayings
4. Framing: landscape openers, closers and dramatic fragments, writing about the document
5. Inflation: significance, promotional words, -ing clauses, vague attribution and association
6. Word habits: copula avoidance, synonym cycling, AI vocabulary, dashes, curly quotes
7. Rhythm: uniform length, repeated openings, forced triads

## 1. Decorated structure

**Bold-labelled bullets.** The most frequent tell in the eval. A short answer becomes `- **Root cause:** ...`, `- **Impact:** ...`. Rewrite as sentences unless there are three or more genuinely parallel items and the profile allows lists; even then drop the bold labels.
Before: "- **Performance:** Queries are faster. - **Reliability:** Fewer timeouts."
After: "Queries use the new index, so they're faster and time out less."

**Headers on short text.** A Slack reply or a 150-word email with `## Summary` / `## Next steps`. Remove the headers and let the first sentence lead.

**Decorative headings and rules.** Title Case In Headings; a horizontal rule between every section; a top-level heading that repeats the page title; arrows (→) or emoji on headings and bullets; a heading written for effect ("The decision, on one screen") instead of naming what the section holds ("How the six options compare"). Use sentence case, drop the rules and decoration, let the title stand once. Keep emoji only where the profile allows (casual Slack) and the other person uses them.

**Heading restated in the first sentence.** `## Performance` followed by "Speed matters." Delete the restating line; start with the content.

**Bold for emphasis.** Bold only what a skimming reader must not miss (a deadline, a destructive step). In docs, a bolded opening sentence per section is house style.

## 2. Padding

**Filler phrases.** "In order to" -> "to"; "due to the fact that" -> "because"; "at this point in time" -> "now"; "has the ability to" -> "can"; "it's worth noting that" -> state the thing.

**Chatbot artefacts.** "Great question!", "Certainly!", "I hope this helps", "Let me know if you have any questions", "Here's a quick summary", "Want me to...?". Delete the wrapper, keep the content. A real offer of help stays only if the user asked to make it. In a rewrite, also no "Here's the rewritten version:" before the text.

**Hedging.** "It could potentially be argued that it might..." -> one qualifier at most, matched to the actual certainty. Stacked qualifiers usually patch an earlier overstatement; fix the claim instead. Keep scope statements, safety notes and real doubt; a single "perhaps" or "tends to" is a human habit.

**Knowledge-limit hedges and guesses.** "As of my last update", "based on available information", "not publicly available", and the guess that follows ("she likely grew up..."). State what the source does not show, or cut the sentence.

## 3. Staging

**Set-up-and-knock-down contrast.** "It's not just about X, it's about Y", "blocked, not just slow", "this isn't a bug, it's a feature gap". Also the split form ("This does not mean X. It means Y.") and the clipped tail ("options come from the selected item, no guessing"). Rewrite as the plain claim. Keep a contrast when the reader currently holds the belief being corrected, or when both halves carry information.
Before: "It's not just a delay; it's a full blocker for QA."
After: "QA is blocked until the migration lands."

**Run-up before the point.** "Let's dive in", "Here's what you need to know", "Here's the thing", "Quick note:", a standalone "Honestly?" or "Look," before a routine claim. Delete the run-up and make the point. "Honestly" inside a casual sentence is ordinary.

**Arguing with no one.** "To be clear", "I'm not saying", "Don't get me wrong", "A tempting approach would be... but", "You might think... but". The text answers an objection or rejects an option nobody raised, often a leftover from drafting. Remove the defence; if it hides a claim, state the claim. Keep an objection the reader actually raised, and an option they would actually weigh.
Before: "Tokens rotate every 24 hours. A tempting approach would be restarting the auth service on a cron, but that would drop sessions. Rotation happens in place."
After: "Tokens rotate in place every 24 hours, and clients refresh without noticing."

**Sayings that sound deep.** "The real question is", "at its core", "what really matters", "the heart of the matter", "X is the language of Y", "X becomes a trap". An ordinary point dressed as a hidden truth. Replace with the specific claim.

**False ranges.** "From API latency to developer happiness" where the ends are not on one scale. Name the actual effects.

## 4. Framing

**Landscape openers** (*weak alone*). "In today's fast-paced...", "When it comes to...", "As we navigate...". Start at the claim.

**Closers and dramatic fragments.** "In short, ...", "The key takeaway is...", "That is the real win.", "That distinction matters.", "Let that sink in."; a final sentence restating the message; a sentence after an example that names what it showed ("This shows the importance of..."); the same closer after every section; a row of fragments ("No config. No setup. No excuses."). End on the last fact or the next step. A short sentence stays when it adds a new fact or consequence.

**Generic positive conclusions.** "The future looks bright", "a big step in the right direction". Replace with the concrete next step or delete.

**Writing about the document instead of its subject.** What the text replaced ("this function was added to replace..."), how it was assembled ("compiled from", "anything unconfirmed is flagged rather than guessed"), a layout the reader can see ("the table below compares"). State the subject. Mention a previous version only in change logs and migration notes; keep a caveat that changes what the reader does; state a convention once, only when the reader cannot infer it. A single page description is *weak alone*.
Before: "The figures below are drawn from each vendor's published pricing; anything we could not confirm is flagged rather than guessed."
After: "Prices are each vendor's published rate. Two vendors publish nothing; call them."

## 5. Inflation

**Significance inflation.** "Pivotal", "marks a shift", "plays a crucial role", "a testament to", "setting the stage for", a stock "Challenges and outlook" section. Replace with what changed.
Before: "This PR marks a significant shift in how we handle token validation."
After: "This PR moves token validation out of the middleware into its own class."

**Promotional words.** "Robust", "seamless", "comprehensive", "cutting-edge", "boasts", "nestled", "renowned", "breathtaking". Name the property instead.

**Superficial -ing clauses.** "..., ensuring better performance and fostering maintainability." Cut or make specific.

**Vague attribution.** "Experts say", "industry reports suggest", a list of outlets that "featured" someone. Cite the source and what it said, or drop the claim.

**Vague association** (*weak alone*). "Associated with", "in connection with", "linked to", "tied to" where the real relationship is known (founded, owns, caused, reports to). Name it. If you don't know it, keep the vague wording rather than guess.

## 6. Word habits

**Copula avoidance** (*weak alone*). "Serves as", "stands as", "functions as", "represents", "boasts", "features", "offers" -> "is", "has".

**Synonym cycling.** "The service... the module... the component..." for one thing. Reuse the plain noun; unusually varied vocabulary is itself a tell.

**AI vocabulary** (*weak alone*). Delve, leverage, utilise, facilitate, additionally, interplay, intricate, enduring, quietly, tapestry, realm, pivotal, seamless. Current models rarely produce most of these; `nab lint` flags the full list. Flag clusters, not single uses. Never swap a flagged word for a neighbour on the same list ("pivotal" -> "crucial"): rewrite the sentence around the fact.

**Dashes.** The final text has no em dashes (—), spaced en dashes ( – ) or double hyphens ( -- ) used as dashes, unless the user's writing sample or the thread uses them; then match that rate. Use a full stop, comma, colon or brackets, or rewrite the sentence. Ranges ("9–5", "pp. 3–7"), code, paths and URLs are untouched.

**Curly quotes** (*weak alone*). Use straight quotes where the medium (Slack, Markdown, code-adjacent docs) uses them.

## 7. Rhythm

**Uniform length** (*weak alone*). Every sentence 15-20 words, every paragraph three sentences. Give space by importance: a one-line paragraph for the point that matters, a longer one for hard reasoning. Treat this as a symptom to check on longer pieces; don't vary length for its own sake.

**Repeated openings** (*weak alone*). Three or more sentences in a row starting with the same word ("We... We... We..."). Merge, or lead with the action. Deliberate repetition for rhythm stays.

**Forced triads** (*weak alone*). Three adjectives, three parallel clauses or three examples where the content has one or two. Real lists of three are fine.

Several patterns, the dash rule and some examples are adapted from [blader/humanizer](https://github.com/blader/humanizer) v4.0.0 (MIT, Siqi Chen), which draws on Wikipedia's "Signs of AI writing".
