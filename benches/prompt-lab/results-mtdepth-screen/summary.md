# Prompt lab summary

p_ai = desklib/ai-text-detector-v1.01 (higher = more AI-like). Facts: exact, case-sensitive token match. Judge: haiku, 1-5. `fact loss` = texts missing at least one fact token / texts.

| variant | n | mean p_ai | median | <0.5 | src mean | fact loss | token ret. | len ratio | same_meaning | reads_human | added_facts | meaning: no major, minor/text (judged) | err |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tx-mt4mix-endfix | 12 | 0.995 | 0.999 | 0% | 0.999 | 1/12 | 99.3% | 1.01 | 4.08 | 3.42 | 1/12 | 92%, 0.50 minor/text (12) | 0 |
| tx-mt4-endfix | 12 | 0.997 | 0.999 | 0% | 0.999 | 1/12 | 99.3% | 0.99 | 4.25 | 3.50 | 0/12 | 92%, 0.42 minor/text (12) | 0 |
| tx-mt5-endfix | 12 | 0.998 | 1.000 | 0% | 0.999 | 2/12 | 98.7% | 0.99 | 4.33 | 3.58 | 1/12 | 100%, 0.33 minor/text (12) | 0 |
| tx-mt2-endfix | 12 | 0.998 | 1.000 | 0% | 0.999 | 1/12 | 99.3% | 1.00 | 4.58 | 3.92 | 0/12 | 100%, 0.17 minor/text (12) | 0 |
| tx-mt3-endfix | 12 | 0.999 | 0.999 | 0% | 0.999 | 1/12 | 99.3% | 0.99 | 4.58 | 3.67 | 0/12 | 100%, 0.42 minor/text (12) | 0 |

## Hops

- tx-mt4mix-endfix: google:zh,edge:ja,google:fi,edge:en,claude-cli:fix
- tx-mt4-endfix: google:zh,google:ja,google:fi,google:en,claude-cli:fix
- tx-mt5-endfix: google:zh,google:ja,google:fi,google:de,google:en,claude-cli:fix
- tx-mt2-endfix: google:zh,google:en,claude-cli:fix
- tx-mt3-endfix: google:zh,google:ja,google:en,claude-cli:fix

## Fact losses

- tx-mt4mix-endfix: linkedin_post-2 [9:30am]
- tx-mt4-endfix: linkedin_post-2 [9:30am]
- tx-mt5-endfix: customer_email-3 [5:00 pm]; linkedin_post-2 [9:30am]
- tx-mt2-endfix: customer_email-3 [5:00 pm]
- tx-mt3-endfix: linkedin_post-2 [9:30am]
