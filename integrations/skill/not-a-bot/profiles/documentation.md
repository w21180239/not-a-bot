# Documentation, wiki page, ADR, runbook

Answer position: each section opens with the sentence a skimming reader needs (what it is, what was decided, what to do); detail follows.
Length: as long as the content needs, uneven by design: detailed where it is tricky, one line where it is standard. Link instead of repeating.
Greeting and sign-off: none. No "In this section we will...".
Formatting: sentence-case headings, phrased as the question the reader has when that reads naturally. Bullets or tables for genuinely parallel items (fields, steps, options). Bolding the opening sentence of a section is fine; no bold-labelled bullet lists. Code examples that run.
Style: active voice with an actor ("We chose X because Y", "The job copies each file to cold storage"). Exact numbers with their denominator. No promotional adjectives.

Voice anchors (synthetic; match the register, don't copy the content):

> The import tool is a command-line script that reads a CSV file and creates one account per row. It validates every row before writing anything, and prints the line number of each row it rejects. Nothing is written if any row fails.

> Over three months, 140 support tickets mentioned slow exports. For 52 of them (37%) the export was larger than the 100 MB limit, which the page never said.

> Each upload gets a row in `upload_log` with its checksum. When a retry arrives with a checksum already in the table, the server returns the stored result instead of writing a second copy. Before this table, duplicates were removed by hand.

Before: "Overview: This comprehensive tool serves as a robust solution, playing a pivotal role in ensuring data integrity across the platform."
After: "What it does: The tool removes upload rows whose file no longer exists, so the next sync can re-upload them."
