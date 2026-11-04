# Laya Loop

A document editor where a small local model (Laya, via [laya-mlx](https://github.com/mizorewww/laya-mlx)) checks every paragraph as you type, and Claude is called only when Laya is unsure. When Claude reviews a paragraph, it also **writes new checks for Laya**. Those checks run locally from then on.

```
typing → Laya runs live checks on changed paragraphs (~4 ms per check, cached)
           ├─ confident hit      → flag in the margin (no Claude call)
           └─ unsure / new issue → queued → you approve → Claude
                                              ├─ rewrite for the paragraph
                                              ├─ verdicts on the rules Laya was unsure about → rules refit
                                              └─ new checks + test examples
                                                     → Laya runs the tests → pass: probation / fail: one reword, else rejected
probation check fires on real text → you approve → Claude verifies → threshold refit → promoted or retired
```

## Run it

Needs Apple Silicon, macOS 14+, and [uv](https://docs.astral.sh/uv/). The first run downloads the Laya checkpoint (about 850 MB).

```bash
uv run laya-loop                      # auto: Claude API if configured, else the claude CLI
uv run laya-loop --backend simulated  # offline stand-in (NOT Claude), to try the loop without credentials
```

Then open http://127.0.0.1:8765.

Claude backends:

| `--backend` | Uses | Setup |
|---|---|---|
| `api` | Anthropic SDK, `claude-opus-5` by default (`LAYA_LOOP_MODEL` to change), structured outputs, server-side refusal fallback | `ANTHROPIC_API_KEY` or `ant auth login` |
| `cli` | `claude -p --safe-mode --tools ""` with a JSON schema (`LAYA_LOOP_CLI_MODEL`, default `opus`) | a logged-in Claude Code |
| `simulated` | canned regex responses, clearly labeled in the UI | none |

Learned checks are saved to `rules.json` (`--rules` to change). Delete that file to start fresh.

## What the measurements showed (and how the design responds)

These were measured with `aac6fef/laya-mlx` on an M4 Max:

- **Negated questions fail.** "Is there a statistic with no source?" couldn't separate sourced from unsourced text. "Does the text name a source for its numbers?" could. Claude's authoring guide forbids negation and uses `direction: "below"` to catch an absence.
- **Probabilities aren't calibrated.** Hostile paragraphs scored 0.13–0.48 and neutral ones about 0.03, so a fixed 0.5 cutoff misses everything. Each check therefore learns its own threshold from its test examples (`validator.py`), and is rejected if no threshold separates them with at least 85% accuracy and a 0.10 margin.
- **The multilingual checkpoint separated almost nothing** on these questions, so the English one is the default.
- **The general "is something wrong here?" detector is weak.** It caught 3 of 5 bad paragraphs with no false alarms. It only routes new kinds of issue to Claude, and there's always a manual "Ask Claude" button.

## Safety and cost guards

- Nothing is sent to Claude without your click. The queue shows the exact text that will be sent. Auto-sending probation verifications is an opt-in toggle.
- Text Claude has already reviewed or written never auto-escalates again. Each paragraph cools down for 10 minutes after an escalation, the approval queue is capped at 5, and there's an hourly budget of 12 calls.
- Claude's checks can only ever produce a probability. They're schema-validated, capped at 2 per review and 25 live checks overall, and must pass Laya's tests before they run at all.
- A probation check needs to match Claude on 3 or more real paragraphs (at least 75% of the last 5) to become active. With 4 or more verdicts and a match rate under 50%, it's retired.
- Every verdict from Claude on real text becomes a labeled example, and the rule's threshold is refit when it still separates.

## Layout

```
laya_loop/
  engine.py      Laya wrapper + (text, question) cache; reduces any answer to one 0-1 signal
  validator.py   test-before-activate: learn a threshold from examples, or reject
  registry.py    persistent check library and lifecycle
  escalation.py  when to ask Claude, and the loop guards
  claude.py      prompts, JSON schemas, API / CLI / simulated backends
  controller.py  the loop itself
  server.py      FastAPI app
  static/        the editor UI
tests/           30 tests using a fake Laya and a fake Claude
```

```bash
uv run pytest -q
```
