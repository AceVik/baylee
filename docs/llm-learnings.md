# LLM Learnings — baylee

Running log of what works when delegating card implementations to local
LLMs (hardware: MacBook M1 Max 64 GB — at most ONE local model active at a
time). Maintained by the orchestrator; entries dated, newest first.

## Process rules (baseline)

1. First batch per model is verified card-by-card; afterwards only special
   cases (layers, copy, replacement, multi-choice cards) are spot-checked.
2. Fixes are applied by the orchestrator, never by the LLM — but every fix
   is analyzed for a prompt improvement and recorded below.
3. Each card task gets: the stub header (oracle text), the forge-reference
   script, one similar already-implemented exemplar, and the DSL cookbook
   excerpt for its mechanic class. Nothing else (token budget).

## Model scoreboard

| Model | Verdict | Notes |
|---|---|---|
| (unset) | | first batches pending (M2.S8) |

## Prompt learnings

(append after each batch: error class → prompt rule that prevents it)
