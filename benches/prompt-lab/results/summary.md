# Prompt lab summary

p_ai = desklib/ai-text-detector-v1.01 (higher = more AI-like). Facts: exact, case-sensitive token match. Judge: haiku, 1-5. `fact loss` = texts missing at least one fact token / texts.

| variant | n | mean p_ai | median | <0.5 | src mean | fact loss | token ret. | len ratio | same_meaning | reads_human | added_facts | err |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tx-standard@r1 | 30 | 0.344 | 0.218 | 73% | 0.999 | 3/30 | 99.0% | 0.69 | 3.80 | 3.93 | 6/30 | 0 |
| v10-oc-light@r1 | 30 | 0.350 | 0.248 | 77% | 0.999 | 1/30 | 99.8% | 0.70 | 3.70 | 4.00 | 4/30 | 0 |
| tx-standard-fix | 30 | 0.361 | 0.185 | 63% | 0.999 | 0/30 | 100.0% | 0.71 | 3.70 | 3.90 | 3/30 | 0 |
| tx-standard | 30 | 0.391 | 0.254 | 63% | 0.999 | 1/30 | 99.8% | 0.69 | 3.87 | 3.97 | 3/30 | 0 |
| tx-standard-fix@r1 | 30 | 0.398 | 0.228 | 63% | 0.999 | 3/30 | 99.0% | 0.69 | 3.87 | 4.00 | 2/30 | 0 |
| tx-standard-fix@r2 | 30 | 0.401 | 0.301 | 63% | 0.999 | 1/30 | 99.8% | 0.73 | 3.77 | 3.97 | 6/30 | 0 |
| v6-oc@r1 | 30 | 0.403 | 0.287 | 63% | 0.999 | 8/30 | 97.1% | 0.64 | 3.77 | 4.00 | 1/30 | 0 |
| v6-oc | 30 | 0.407 | 0.318 | 70% | 0.999 | 7/30 | 97.1% | 0.62 | 3.63 | 3.97 | 2/30 | 0 |
| v10-oc-light | 30 | 0.410 | 0.320 | 60% | 0.999 | 4/30 | 98.8% | 0.69 | 3.77 | 3.97 | 3/30 | 0 |
| v12-oc-3hop | 30 | 0.415 | 0.383 | 63% | 0.999 | 8/30 | 97.5% | 0.70 | 3.80 | 4.00 | 4/30 | 0 |
| v11-oc-factfix@r1 | 30 | 0.416 | 0.330 | 63% | 0.999 | 0/30 | 100.0% | 0.71 | 3.63 | 3.97 | 4/30 | 0 |
| v9-oc-fold | 30 | 0.429 | 0.306 | 63% | 0.999 | 7/30 | 97.5% | 0.63 | 3.70 | 3.90 | 2/30 | 0 |
| v14-oc-cut-factfix@r1 | 30 | 0.429 | 0.317 | 63% | 0.999 | 1/30 | 99.5% | 0.65 | 3.73 | 4.03 | 2/30 | 0 |
| v12-oc-3hop@r1 | 30 | 0.435 | 0.366 | 57% | 0.999 | 3/30 | 99.0% | 0.70 | 3.70 | 3.97 | 6/30 | 0 |
| v10-oc-light@r2 | 30 | 0.436 | 0.313 | 60% | 0.999 | 2/30 | 99.5% | 0.70 | 3.73 | 3.97 | 3/30 | 0 |
| v13-oc-3hop-factfix@r1 | 30 | 0.466 | 0.479 | 50% | 0.999 | 0/30 | 100.0% | 0.71 | 3.83 | 3.97 | 5/30 | 0 |
| v14-oc-cut-factfix | 30 | 0.478 | 0.425 | 63% | 0.999 | 2/30 | 99.0% | 0.63 | 3.67 | 4.00 | 2/30 | 0 |
| v11-oc-factfix | 30 | 0.487 | 0.497 | 50% | 0.999 | 1/30 | 99.5% | 0.68 | 3.77 | 3.97 | 4/30 | 0 |
| v13-oc-3hop-factfix | 30 | 0.511 | 0.495 | 50% | 0.999 | 4/30 | 99.0% | 0.71 | 3.83 | 4.00 | 5/30 | 0 |
| v6-cut | 30 | 0.523 | 0.419 | 53% | 0.999 | 7/30 | 97.3% | 0.64 | 3.63 | 4.03 | 2/30 | 0 |
| v0-oc | 30 | 0.530 | 0.469 | 53% | 0.999 | 2/30 | 99.5% | 0.85 | 4.07 | 4.07 | 4/30 | 0 |
| v0-oc@r2 | 30 | 0.537 | 0.497 | 50% | 0.999 | 0/30 | 100.0% | 0.86 | 4.13 | 4.00 | 2/30 | 0 |
| v8-cut-final-only | 30 | 0.539 | 0.534 | 50% | 0.999 | 7/30 | 97.3% | 0.65 | 3.53 | 4.00 | 3/30 | 0 |
| v7-direct-cut | 30 | 0.559 | 0.543 | 50% | 0.999 | 8/30 | 96.1% | 0.59 | 3.57 | 4.00 | 0/30 | 0 |
| v0-oc@r1 | 30 | 0.669 | 0.782 | 27% | 0.999 | 5/30 | 98.8% | 0.84 | 4.00 | 4.00 | 7/30 | 0 |
| v1-direct | 30 | 0.710 | 0.832 | 27% | 0.999 | 2/30 | 98.8% | 0.75 | 3.97 | 4.00 | 2/30 | 0 |
| v0-current | 30 | 0.736 | 0.843 | 20% | 0.999 | 1/30 | 99.8% | 0.85 | 4.17 | 4.00 | 2/30 | 0 |
| v5-ja-pivot | 30 | 0.786 | 0.872 | 17% | 0.999 | 1/30 | 99.8% | 0.86 | 4.03 | 3.97 | 2/30 | 0 |
| v2-human-features | 30 | 0.805 | 0.948 | 17% | 0.999 | 1/30 | 99.8% | 0.86 | 4.13 | 4.00 | 3/30 | 0 |
| v4-haiku-final | 30 | 0.814 | 0.973 | 13% | 0.999 | 5/30 | 98.8% | 0.88 | 4.33 | 4.00 | 2/30 | 0 |
| v3-literal-then-humanize | 30 | 0.924 | 0.991 | 7% | 0.999 | 0/30 | 100.0% | 0.92 | 4.43 | 4.03 | 1/30 | 0 |

## Pooled over samples

Mean over samples; range = min-max of the per-sample means.

| variant | samples | mean p_ai (range) | <0.5 | fact loss (range) | same_meaning | reads_human |
|---|---|---|---|---|---|---|
| tx-standard | 2 | 0.367 (0.344-0.391) | 68% | 2.0 (1-3) / 30 | 3.83 | 3.95 |
| tx-standard-fix | 3 | 0.387 (0.361-0.401) | 63% | 1.3 (0-3) / 30 | 3.78 | 3.96 |
| v10-oc-light | 3 | 0.398 (0.350-0.436) | 66% | 2.3 (1-4) / 30 | 3.73 | 3.98 |
| v6-oc | 2 | 0.405 (0.403-0.407) | 67% | 7.5 (7-8) / 30 | 3.70 | 3.98 |
| v12-oc-3hop | 2 | 0.425 (0.415-0.435) | 60% | 5.5 (3-8) / 30 | 3.75 | 3.98 |
| v11-oc-factfix | 2 | 0.452 (0.416-0.487) | 57% | 0.5 (0-1) / 30 | 3.70 | 3.97 |
| v14-oc-cut-factfix | 2 | 0.454 (0.429-0.478) | 63% | 1.5 (1-2) / 30 | 3.70 | 4.02 |
| v13-oc-3hop-factfix | 2 | 0.488 (0.466-0.511) | 50% | 2.0 (0-4) / 30 | 3.83 | 3.98 |
| v0-oc | 3 | 0.579 (0.530-0.669) | 43% | 2.3 (0-5) / 30 | 4.07 | 4.02 |

## Hops

- tx-standard@r1: opencode-cli:zh,opencode-cli:en
- v10-oc-light@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- tx-standard-fix: opencode-cli:zh,opencode-cli:en,opencode-cli:fix
- tx-standard: opencode-cli:zh,opencode-cli:en
- tx-standard-fix@r1: opencode-cli:zh,opencode-cli:en,opencode-cli:fix
- tx-standard-fix@r2: opencode-cli:zh,opencode-cli:en,opencode-cli:fix
- v6-oc@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v6-oc: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v10-oc-light: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v12-oc-3hop: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>ja, opencode-go/deepseek-v4.1-flash:ja>en
- v11-oc-factfix@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en, opencode-go/deepseek-v4.1-flash:en>en
- v9-oc-fold: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v14-oc-cut-factfix@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en, opencode-go/deepseek-v4.1-flash:en>en
- v12-oc-3hop@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>ja, opencode-go/deepseek-v4.1-flash:ja>en
- v10-oc-light@r2: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v13-oc-3hop-factfix@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>ja, opencode-go/deepseek-v4.1-flash:ja>en, opencode-go/deepseek-v4.1-flash:en>en
- v14-oc-cut-factfix: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en, opencode-go/deepseek-v4.1-flash:en>en
- v11-oc-factfix: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en, opencode-go/deepseek-v4.1-flash:en>en
- v13-oc-3hop-factfix: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>ja, opencode-go/deepseek-v4.1-flash:ja>en, opencode-go/deepseek-v4.1-flash:en>en
- v6-cut: sonnet:en>zh, sonnet:zh>en
- v0-oc: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v0-oc@r2: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v8-cut-final-only: sonnet:en>zh, sonnet:zh>en
- v7-direct-cut: sonnet:en>en
- v0-oc@r1: opencode-go/deepseek-v4.1-flash:en>zh, opencode-go/deepseek-v4.1-flash:zh>en
- v1-direct: sonnet:en>en
- v0-current: sonnet:en>zh, sonnet:zh>en
- v5-ja-pivot: sonnet:en>ja, sonnet:ja>en
- v2-human-features: sonnet:en>zh, sonnet:zh>en
- v4-haiku-final: sonnet:en>zh, haiku:zh>en
- v3-literal-then-humanize: sonnet:en>zh, sonnet:zh>en

## Fact losses

- tx-standard@r1: documentation-1 [Kauri, Analytics]; documentation-5 [12]; documentation-6 [$24.95]
- v10-oc-light@r1: documentation-2 [$4,800]
- tx-standard-fix: none
- tx-standard: slack_update-5 [`v2.4.0`]
- tx-standard-fix@r1: customer_email-1 [Ms, November]; marketing_blurb-3 [1]; documentation-2 [$4,800]
- tx-standard-fix@r2: slack_update-5 [`v2.4.0`]
- v6-oc@r1: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-3 [1]; marketing_blurb-5 [Queenstown]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v6-oc: customer_email-4 [100%, Queenstown]; customer_email-6 [Waikato]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; documentation-2 [$4,800]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v10-oc-light: slack_update-3 [14]; marketing_blurb-3 [1]; documentation-6 [$24.95]; linkedin_post-2 [New, Zealand]
- v12-oc-3hop: customer_email-3 [Ms]; customer_email-4 [100%]; slack_update-2 [3]; slack_update-3 [14, 6]; slack_update-5 [CLI]; marketing_blurb-3 [1]; documentation-5 [Updated]; linkedin_post-3 [9, 24]
- v11-oc-factfix@r1: none
- v9-oc-fold: customer_email-4 [100%]; customer_email-6 [Waikato]; slack_update-1 [Queenstown]; slack_update-2 [2.8.0]; marketing_blurb-6 [New, Zealand]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v14-oc-cut-factfix@r1: documentation-1 [Kauri, Analytics]
- v12-oc-3hop@r1: documentation-5 [12]; linkedin_post-1 [New, Zealand]; linkedin_post-6 [4]
- v10-oc-light@r2: customer_email-4 [Queenstown]; marketing_blurb-3 [1]
- v13-oc-3hop-factfix@r1: none
- v14-oc-cut-factfix: marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]
- v11-oc-factfix: linkedin_post-2 [New, Zealand]
- v13-oc-3hop-factfix: customer_email-3 [Ms]; slack_update-2 [3]; slack_update-5 [CLI]; documentation-5 [Updated]
- v6-cut: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-2 [Top, South]; marketing_blurb-5 [Queenstown]; marketing_blurb-6 [New, Zealand]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v0-oc: customer_email-4 [100%]; slack_update-3 [14]
- v0-oc@r2: none
- v8-cut-final-only: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-2 [Top]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v7-direct-cut: customer_email-4 [Queenstown]; customer_email-6 [Waikato]; marketing_blurb-2 [Top, South]; marketing_blurb-6 [New, Zealand]; documentation-1 [Kauri, Analytics]; documentation-6 [Waikato, Paddock, Sensor, Kit]; linkedin_post-1 [New, Zealand]; linkedin_post-2 [New, Zealand]
- v0-oc@r1: customer_email-4 [100%]; customer_email-6 [Waikato]; slack_update-2 [3]; slack_update-5 [CLI]; documentation-5 [Updated]
- v1-direct: customer_email-6 [Waikato]; documentation-6 [Waikato, Paddock, Sensor, Kit]
- v0-current: documentation-5 [Updated]
- v5-ja-pivot: marketing_blurb-2 [$59.90]
- v2-human-features: documentation-5 [Updated]
- v4-haiku-final: customer_email-3 [Portal]; slack_update-2 [3]; marketing_blurb-2 [Top]; documentation-2 [$4,800]; linkedin_post-6 [4]
- v3-literal-then-humanize: none
