# Prompt lab summary

p_ai = desklib/ai-text-detector-v1.01 (higher = more AI-like). Facts: exact, case-sensitive token match. Judge: haiku, 1-5. `fact loss` = texts missing at least one fact token / texts.

| variant | n | mean p_ai | median | <0.5 | src mean | fact loss | token ret. | len ratio | same_meaning | reads_human | added_facts | err |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| v6-oc | 30 | 0.407 | 0.318 | 70% | 0.999 | 7/30 | 97.1% | 0.62 | 3.63 | 3.97 | 2/30 | 0 |
| v10-oc-light | 30 | 0.410 | 0.320 | 60% | 0.999 | 4/30 | 98.8% | 0.69 | 3.77 | 3.97 | 3/30 | 0 |
| v9-oc-fold | 30 | 0.429 | 0.306 | 63% | 0.999 | 7/30 | 97.5% | 0.63 | 3.70 | 3.90 | 2/30 | 0 |
| v6-cut | 30 | 0.523 | 0.419 | 53% | 0.999 | 7/30 | 97.3% | 0.64 | 3.63 | 4.03 | 2/30 | 0 |
| v0-oc | 30 | 0.530 | 0.469 | 53% | 0.999 | 2/30 | 99.5% | 0.85 | 4.07 | 4.07 | 4/30 | 0 |
| v8-cut-final-only | 30 | 0.539 | 0.534 | 50% | 0.999 | 7/30 | 97.3% | 0.65 | 3.53 | 4.00 | 3/30 | 0 |
| v7-direct-cut | 30 | 0.559 | 0.543 | 50% | 0.999 | 8/30 | 96.1% | 0.59 | 3.57 | 4.00 | 0/30 | 0 |
| v1-direct | 30 | 0.710 | 0.832 | 27% | 0.999 | 2/30 | 98.8% | 0.75 | 3.97 | 4.00 | 2/30 | 0 |
| v0-current | 30 | 0.736 | 0.843 | 20% | 0.999 | 1/30 | 99.8% | 0.85 | 4.17 | 4.00 | 2/30 | 0 |
| v5-ja-pivot | 30 | 0.786 | 0.872 | 17% | 0.999 | 1/30 | 99.8% | 0.86 | 4.03 | 3.97 | 2/30 | 0 |
| v2-human-features | 30 | 0.805 | 0.948 | 17% | 0.999 | 1/30 | 99.8% | 0.86 | 4.13 | 4.00 | 3/30 | 0 |
| v4-haiku-final | 30 | 0.814 | 0.973 | 13% | 0.999 | 5/30 | 98.8% | 0.88 | 4.33 | 4.00 | 2/30 | 0 |
| v3-literal-then-humanize | 30 | 0.924 | 0.991 | 7% | 0.999 | 0/30 | 100.0% | 0.92 | 4.43 | 4.03 | 1/30 | 0 |

## Hops

- v6-oc: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v10-oc-light: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v9-oc-fold: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v6-cut: sonnet:en>zh, sonnet:zh>en
- v0-oc: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v8-cut-final-only: sonnet:en>zh, sonnet:zh>en
- v7-direct-cut: sonnet:en>en
- v1-direct: sonnet:en>en
- v0-current: sonnet:en>zh, sonnet:zh>en
- v5-ja-pivot: sonnet:en>ja, sonnet:ja>en
- v2-human-features: sonnet:en>zh, sonnet:zh>en
- v4-haiku-final: sonnet:en>zh, haiku:zh>en
- v3-literal-then-humanize: sonnet:en>zh, sonnet:zh>en

## Fact losses

- v6-oc: customer_email-4 [100%, Queenstown]; customer_email-6 [Waikato]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; documentation-2 [$4,800]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v10-oc-light: slack_update-3 [14]; marketing_blurb-3 [1]; documentation-6 [$24.95]; linkedin_post-2 [New, Zealand]
- v9-oc-fold: customer_email-4 [100%]; customer_email-6 [Waikato]; slack_update-1 [Queenstown]; slack_update-2 [2.8.0]; marketing_blurb-6 [New, Zealand]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v6-cut: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-2 [Top, South]; marketing_blurb-5 [Queenstown]; marketing_blurb-6 [New, Zealand]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v0-oc: customer_email-4 [100%]; slack_update-3 [14]
- v8-cut-final-only: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-2 [Top]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v7-direct-cut: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-2 [Top, South]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; documentation-6 [Waikato, Paddock, Sensor, Kit]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v1-direct: customer_email-6 [Waikato]; documentation-6 [Waikato, Paddock, Sensor, Kit]
- v0-current: documentation-5 [Updated]
- v5-ja-pivot: marketing_blurb-2 [$59.90]
- v2-human-features: documentation-5 [Updated]
- v4-haiku-final: customer_email-3 [Portal]; slack_update-2 [3]; marketing_blurb-2 [Top]; documentation-2 [$4,800]; linkedin_post-6 [4]
- v3-literal-then-humanize: none
