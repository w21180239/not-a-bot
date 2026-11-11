# Prompt lab summary

p_ai = desklib/ai-text-detector-v1.01 (higher = more AI-like). Facts: exact, case-sensitive token match. Judge: haiku, 1-5. `fact loss` = texts missing at least one fact token / texts.

| variant | n | mean p_ai | median | <0.5 | src mean | fact loss | token ret. | len ratio | same_meaning | reads_human | added_facts | meaning: no major, minor/text (judged) | err |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tx-llm3-fix | 12 | 0.329 | 0.111 | 67% | 0.999 | 0/12 | 100.0% | 0.73 | nan | nan | 0/0 | 92%, 0.92 minor/text (12) | 0 |
| tx-mix-perhop | 12 | 0.392 | 0.275 | 58% | 0.999 | 3/12 | 97.3% | 0.83 | nan | nan | 0/0 | 91%, 3.64 minor/text (11) | 0 |
| tx-llm-mt-llm-endfix | 12 | 0.446 | 0.252 | 58% | 0.999 | 0/12 | 100.0% | 0.75 | nan | nan | 0/0 | 100%, 0.75 minor/text (12) | 0 |
| tx-llm-mt-llm-perhop | 12 | 0.535 | 0.556 | 42% | 0.999 | 0/12 | 100.0% | 0.73 | nan | nan | 0/0 | 100%, 0.50 minor/text (12) | 0 |
| tx-mt-out-perhop | 12 | 0.580 | 0.583 | 50% | 0.999 | 0/12 | 100.0% | 0.73 | nan | nan | 0/0 | 100%, 0.25 minor/text (12) | 0 |
| tx-light-llm-out | 12 | 0.607 | 0.685 | 33% | 0.999 | 3/12 | 98.0% | 0.80 | nan | nan | 0/0 | 92%, 2.67 minor/text (12) | 0 |
| tx-light-perhop | 12 | 0.885 | 0.993 | 8% | 0.999 | 2/12 | 98.7% | 0.98 | nan | nan | 0/0 | 100%, 2.00 minor/text (12) | 0 |
| tx-light-d1-perhop | 12 | 0.971 | 0.999 | 0% | 0.999 | 2/12 | 97.3% | 1.01 | nan | nan | 0/0 | 90%, 2.10 minor/text (10) | 0 |

## Hops

- tx-llm3-fix: opencode-cli:zh,opencode-cli:ja,opencode-cli:en,opencode-cli:fix
- tx-mix-perhop: opencode-cli:zh,opencode-cli:ja,google:fi,opencode-cli:fix,edge:en,opencode-cli:fix
- tx-llm-mt-llm-endfix: opencode-cli:zh,google:ja,opencode-cli:en,opencode-cli:fix
- tx-llm-mt-llm-perhop: opencode-cli:zh,google:ja,opencode-cli:fix,opencode-cli:en,opencode-cli:fix
- tx-mt-out-perhop: google:zh,opencode-cli:fix,opencode-cli:en,opencode-cli:fix
- tx-light-llm-out: opencode-cli:zh,google:en,opencode-cli:fix
- tx-light-perhop: zh,opencode-cli:fix,ja,opencode-cli:fix,en,opencode-cli:fix
- tx-light-d1-perhop: zh,opencode-cli:fix,en,opencode-cli:fix

## Fact losses

- tx-llm3-fix: none
- tx-mix-perhop: customer_email-3 [5:00 pm, Ms]; documentation-5 [Updated]; linkedin_post-2 [9:30am]
- tx-llm-mt-llm-endfix: none
- tx-llm-mt-llm-perhop: none
- tx-mt-out-perhop: none
- tx-light-llm-out: customer_email-3 [5:00 pm]; documentation-5 [Updated]; linkedin_post-2 [9:30am]
- tx-light-perhop: customer_email-3 [5:00 pm]; linkedin_post-2 [9:30am]
- tx-light-d1-perhop: customer_email-3 [5:00 pm, Portal, Onboarding]; linkedin_post-2 [9:30am]
