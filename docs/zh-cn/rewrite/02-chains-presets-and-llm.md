**简体中文** · [English](../../en/rewrite/02-chains-presets-and-llm.md)

# 往返翻译 — 链、预设与 LLM 跳

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

一次往返就是一串跳。机器翻译改用词、保留句子结构；LLM 跳可以重建句子。链把两者混用，预设是测过并命名的链。

## 链怎么工作

一跳写作 `[engine:]lang`。链是逗号分隔的跳列表，引擎和语言可以任意排列：

```bash
not-a-bot rewrite "<text>" --from en --chain "opencode-cli:zh,google:ja,edge:en"
```

- 跳里没写引擎：由自动 Provider 链处理（最多 `race_width` 个 Provider 并发竞速）。
- 写了引擎：该跳优先用这个引擎（见[优先引擎与回退](#优先引擎与回退)）。
- 结果必须落在原语言。最后一跳不是原语言时，自动补一跳回到原语言。
- `<llm>:fix` 是修复跳，留在当前语言（见[修复跳](#修复跳)）。

会展开成链的快捷参数：

| 参数 | 展开为 |
| --- | --- |
| `--depth 1` | 一次机器翻译经 zh 出去再回来 |
| `--depth 2` | zh、ja、回来 |
| `--depth 3` | zh、ja、fi、回来 |
| `--pivot ja` | 一跳经 ja 出去再回来 |
| `--preset <name>` | 下表里的具名链 |

中文原文用 en 代替 zh：depth 3 为 en、ja、fi，再回到中文。

优先级，先匹配者生效：`--chain`、`--preset`、`--depth`、`--pivot`，最后是配置里的 `rewrite_preset`（默认 `standard`）。

## 预设

| 预设 | 跳（英文原文） | 需要订阅 | 典型延迟 |
| --- | --- | --- | --- |
| `light` | zh、ja、en：出去两跳机器翻译 | 否 | 约 1 秒 |
| `standard` | LLM 译到 zh，LLM 再回 en，最后一次只修事实的修复跳（`<llm>:fix`）；前两跳"像人一样翻译并改写" | 是 | 8 到 14 秒，另加一次修复调用 |
| `strong` | LLM 到 zh，LLM 再回 en；没有修复跳 | 是 | 8 到 14 秒 |

人性化跳会删掉只表达感受、承诺或客套的句子。`standard` 随后只修硬事实：名字、地点、产品、数字、日期、金额、币种、ID、版本、路径、代码片段、否定和限定词。修复跳会拿到事实检查列出的缺失片段，并且从不恢复被删的感受或承诺句。`strong` 不带修复跳：改写最远，检测器分数最低，但可能丢一条事实，`missingFacts` 一定会报告。`standard` 和 `strong` 需要 LLM 引擎：`rewrite_llm` 不是 LLM 引擎时返回 `INVALID_INPUT`。

没有任何方法能让文本稳定地检测不出，not-a-bot 也不做这个承诺。目标是读起来像人写的文字，事实保持不变。`--depth 1-3` 和 `--chain` 可以手动搭出其他链。其他预设名会返回 `INVALID_INPUT`，并列出 `light, standard, strong`。

真实二进制实测（30 段英文、desklib 检测器、每段 2 到 3 次采样，`benches/prompt-lab`）：早先的提示词平均 `p_ai` 0.579，平均 2.3 段丢事实片段；`standard` 0.387 / 1.3；`strong` 0.367 / 2.0。

默认值写在 `config.toml`：

```toml
rewrite_preset = "standard"  # 默认：standard
rewrite_llm = "claude-cli"   # 默认：opencode-cli
```

环境变量：`NOT_A_BOT_REWRITE_PRESET`、`NOT_A_BOT_REWRITE_LLM`。

## LLM Provider 与额度

两个 Provider 调用本机的 agent CLI，不需要 API key，用的是你已经在付费的订阅：

| Provider | 执行 | 额度 | 默认模型 |
| --- | --- | --- | --- |
| `claude-cli` | `claude -p` | 你的 Claude 订阅 | haiku |
| `opencode-cli` | `opencode run` | 你的 OpenCode 额度 | `opencode-go/deepseek-v4.1-flash` |

```toml
[provider.claude-cli]
model = "haiku"
bin = "claude"

[provider.opencode-cli]
model = "opencode-go/deepseek-v4.1-flash"
bin = "opencode"
```

它们比网页端点慢（每跳几秒），每跳会花一点额度。`auto` 从不选它们：只有被点名时才运行，即出现在链的某一跳、预设或 `--provider` 里。

## LLM 跳收到什么

- 这一跳的指令：翻译，并像人一样改写。
- 往返里还会收到原文。这一跳先用最小的改动补回前面机器翻译跳丢掉的硬事实，再做改写。
- 后面的 LLM 跳还会收到上一个 LLM 跳的结果作为历史。
- 受保护片段原样传入。LLM 跳不使用占位符：提示词里列出要保留的片段，翻译完由 not-a-bot 检查输出。片段丢了会加一条 `preserve: <provider> dropped <span>` 警告。机器翻译跳仍然用占位符遮蔽。

不用自己写正则，直接用具名模式：`--preserve @code`（行内和围栏代码）和 `--preserve @url`。在 Windows 上，npm 的 `.cmd` 外壳经 cmd.exe 运行；Python 或 Node 不经 shell 启动 not-a-bot 时，cmd.exe 会吞掉 `` `[^`]*` `` 这类正则里的 `^`。具名模式避开这个问题。

## 修复跳

`<llm>:fix`（如 `opencode-cli:fix`，或 `standard` 的最后一跳）让文本留在当前语言，只请 LLM 返回一个 JSON 小编辑列表，而不是一份新文本。它对照原文只修事实：名字、地点、产品、数字、日期、金额、币种、ID、版本、路径、代码片段、否定和限定词；会拿到事实检查列出的缺失片段；从不恢复被人性化跳删掉的、只表达感受、承诺或客套的句子，其中的事实并入最近保留的句子：

```json
[{"from": "Aloha's team", "to": "Aroha's team", "why": "name respelled"}]
```

not-a-bot 只应用精确且很小的编辑：

- `from` 必须在当前文本里恰好出现一次，不能为空，长度至多取 40 个字符与文本 25% 中较大者。
- `to` 至多是 `from` 长度的三倍再加 80 个字符。
- 补回丢失事实（名字、数字或其他被检查列出的片段）的编辑一律应用，不占预算。
- 其余编辑（`kind: meaning`）共用一份预算：与原文的实词重合度，相对做完事实编辑后的文本，至多升高 0.10。许多小编辑加起来也无法把原文抄回来。

每一跳会报告每条编辑的结果：

```json
"edits": [
  {"from": "Aloha's team", "to": "Aroha's team", "why": "name respelled", "kind": "fact", "applied": true},
  {"from": "…", "to": "…", "why": "…", "kind": "meaning", "applied": false, "reason": "copies the original back"}
]
```

跳过的原因包括 `from not found`、`from not unique`、`no change`、`to too long for from` 和 `copies the original back`。

在下面的语料上，修复跳减少了没有它的同一条链的事实丢失，并保留了大部分改写。

## 事实检查

每次往返，不论哪种链，都会拿结果与原文比对，并把缺失的片段列在 `missingFacts`：

- 代码片段、URL、路径、工单 ID、@提及和数字：总是检查。数字按数位比较，`18,900` 与 `18900` 相同，`4:50pm` 与 `4:50 PM` 相同。
- 大写开头的人名：仅在两段文本都是拉丁字母时检查，按整词匹配，所以 `Tomi` 不能证明 `Tom` 还在。句中人名区分大小写；句首人名按不区分大小写检查，但若它是常见句首词，或在原文别处以小写出现，则不检查，所以与这类词同形的人名可能漏检。

列表非空时还会加一条 `fact check: missing <list>` 警告。事实检查只报告，不修复。读完后再用修复跳重跑，或手动补回。

## 优先引擎与回退

链里点名的机器翻译引擎是"优先"，不是"钉死"。遇到可重试错误（`RATE_LIMITED`、`TIMEOUT`、`NETWORK_ERROR`、`QUOTA_EXHAUSTED`），这一跳会改用自动链里的另一个引擎，并避开相邻两跳用过的引擎，同时加一条警告：

```
hop 2: edge failed (RATE_LIMITED), served by google
```

LLM 跳从不回退：换一个模型会改变结果，所以失败时直接返回错误。

## 检测器打分、多次取优与目标分

LLM 跳每次输出都不一样，所以再跑一次可能读起来不同。配上打分器后，not-a-bot 按检测器分数在多次运行之间挑选，但有一条硬规则：保住全部事实的运行永远胜过丢了事实的运行，分数再低也不行。它用来做 benchmark 和调 preset；本地检测器每次调用都要加载模型，放在实时请求里太慢太重。

```bash
not-a-bot rewrite "<text>" --from en --preset standard --best-of 3 --scorer "python score.py" --format json
not-a-bot rewrite "<text>" --from en --target-score 0.4 --scorer "python score.py" --format json
```

- `--scorer "<command>"`（配置项 `rewrite_scorer`）：not-a-bot 把每个结果写进该命令的 stdin，从 stdout 最后一个非空行读一个 0 到 1 的分数（越高越像 AI）。命令按词切分（引号成组，反斜杠保持原样），不经过 shell；`PATH` 上的程序按 CLI provider 的同一套规则解析，包括 `.cmd` 包装脚本。每次调用的时限是 `--timeout-ms`，但不少于 120 秒，因为本地检测器每次调用都要加载模型。
- 打分器失败、超时，或输出的不是 [0, 1] 内的数字时，加一条 `scorer: run <n> (<preset>) unscored: <reason>` 警告，这次运行记为未打分。所有运行都没有分数时，结果与不配打分器相同。
- `--best-of N`（1-5，默认 1）把规划好的链独立跑 N 次。挑选顺序：先看没有丢事实的（都丢了就看丢得最少的），再看分数最低的，未打分的排在已打分的后面。N 大于 1 时必须配打分器，否则返回 `INVALID_INPUT`。
- `--target-score X`（0-1，需要打分器）：先跑请求的链（带上 `--best-of`），再依次跑 `rewrite_ladder`（默认 `["standard", "strong"]`）里还没试过的预设。只要目前最好的运行分数不高于 X 就停下；全部试完仍未达到，就返回总体最好的运行，并加一条 `target score not met` 警告。

响应新增 `score`（选中运行的分数，没有可用打分器时为 `null`）、`scorer`（程序名，不含参数），运行多于一次时还有 `candidates`：每次运行一条 `{presetOrChain, score, missingFacts}`，其中 `missingFacts` 是个数。只返回选中运行的文本和各跳。

```toml
rewrite_scorer = "python score.py"
rewrite_ladder = ["standard", "strong"]
```

分数只是某一个检测器对某一段文本的测量结果。汇报时注明检测器名称；换一个检测器结论可能不同，分数低也不代表文本无法被检测出来。

## 怎么选

| 场景 | 选择 |
| --- | --- |
| 把 AI 味的文字改得像人写的，事实不丢 | `--preset standard` |
| 改写力度最大，可能丢一条事实（见 `missingFacts`） | `--preset strong` |
| 没有订阅，或者要快 | `--preset light`（默认） |
| 机器翻译，再多改写一点 | `--depth 3`；事实丢失涨得比改写快 |
| 带代码或 URL 的文本 | 任何预设都加 `--preserve @code --preserve @url` |
| 合同、法律、读者要核对的数字 | 不要往返 |

每次运行后读 `missingFacts` 和 `warnings`，并如实报告。

## 导航

- ← [往返翻译概览](OVERVIEW.md)
- → [选择中转语言和跳数](01-pivot-choice.md)
- ↑ [文档索引](../README.md)
