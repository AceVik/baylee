#!/usr/bin/env python3
"""What the four lanes share: the repo, the work directory, and the two ways
of reaching a model.

A *lane* is one cheap model doing one job — writing cards, or writing the
engine test for a card somebody else wrote. There are two of each because
the two models are reached in completely different ways, and that difference
is the whole cost story:

- **DeepSeek** answers an Anthropic-shaped HTTP API. It browses nothing, so
  the context it would have read for itself is packed and sent as one
  cacheable prefix; thirty cards pay for that prefix once. It is billed per
  token and the bill is small.
- **Gemini** is driven through the `agy` CLI as one agentic session per
  batch. It reads the repository for itself and pays the growing transcript
  on every step. Under the owner's subscription that costs no money at all —
  what it spends is a **time quota that refreshes**, so the planning unit for
  that lane is minutes, not tokens. The token figure is still measured and
  still worth reading, because it is what makes a session slow and what a
  quota is really being spent on.

Every run writes beside its own worklist and never into the repository:
`<worklist-dir>/<stem>-report.md`, `<stem>-answer.md`, `<stem>-tests/…`.
Put the worklist somewhere scratch and the whole run lands there with it.

Nothing here runs cargo. The build belongs to the coordinator — a model that
cannot run it must not be the last reader of what it wrote.
"""

import json
import os
import pathlib
import sqlite3
import subprocess
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]

# Where `agy` keeps one sqlite transcript per session. Read-only, and only
# ever for the measurement below.
GEMINI_CONVERSATIONS = pathlib.Path(
    os.path.expanduser("~/.gemini/antigravity-cli/conversations")
)
GEMINI_MODEL = "gemini-3.8-flash-high"
DEEPSEEK_MODEL = "deepseek-flash[1m]"

DOORS = {
    "cards/lands/": "lands",
    "cards/creatures/": "creatures",
    "cards/artifacts/": "artifacts",
    "cards/enchantments/": "enchantments",
    "cards/instants/": "instants",
    "cards/sorceries/": "sorceries",
    "cards/planeswalkers/": "planeswalkers",
}


def door(path: str) -> str:
    """Which `card_tests` module a card's test belongs in.

    The same taxonomy `cards/` puts the card file behind, because a test
    goes with the card it plays. Anything that matches nothing is a rule
    rather than a card, and `rules` is where those live.
    """
    for key, name in DOORS.items():
        if key in path:
            return name
    return "rules"


def outputs(list_file: str):
    """`(stem, directory)` for everything a run on this worklist produces.

    Derived from the worklist rather than from this script's own location:
    these files used to land next to the code, which is exactly where a
    generated answer must not be.
    """
    p = pathlib.Path(list_file).resolve()
    return p.stem, p.parent


def worklist(list_file: str) -> list[str]:
    """One repo-relative card path per line, blank lines ignored."""
    return [
        line.strip()
        for line in pathlib.Path(list_file).read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


# --------------------------------------------------------------- DeepSeek


def deepseek_key() -> str:
    """The API key, from the environment or from the opencode auth file.

    Returned, never printed: it has been in a terminal once and that was
    once too often.
    """
    key = os.environ.get("DEEPSEEK_API_KEY")
    if key:
        return key
    auth = pathlib.Path.home() / ".local/share/opencode/auth.json"
    if not auth.exists():
        raise SystemExit(
            "no DeepSeek key: set DEEPSEEK_API_KEY, or log in with opencode "
            f"so that {auth} exists"
        )
    try:
        return json.loads(auth.read_text(encoding="utf-8"))["deepseek"]["key"]
    except (KeyError, json.JSONDecodeError) as exc:
        raise SystemExit(f"{auth} has no usable `deepseek.key`: {exc}") from exc


def pack(files: list[str], examples: list[str] = (), heading: str = "") -> str:
    """The context the model would have browsed, as one cacheable prefix.

    An example that no longer exists is skipped rather than fatal: the pool
    is re-filed by codegen and a moved exemplar should cost a worse prompt,
    not a failed batch. A missing *vocabulary* file is fatal, because a lane
    that quietly sends half the DSL invents the other half.
    """
    parts = [heading or "# Der Autorenvertrag und der ganze DSL-Wortschatz\n"]
    for rel in files:
        parts.append(
            f"\n## `{rel}`\n\n```\n{(ROOT / rel).read_text(encoding='utf-8')}\n```\n"
        )
    if examples:
        parts.append("\n# Fertige Dateien als Vorbild\n")
        for rel in examples:
            p = ROOT / rel
            if p.exists():
                parts.append(
                    f"\n## `{rel}`\n\n```rust\n{p.read_text(encoding='utf-8')}\n```\n"
                )
    return "".join(parts)


def ask_deepseek(prefix: str, rules: str, body_text: str, max_tokens: int) -> str:
    """One question, with `prefix` marked cacheable.

    `curl` rather than a client library so the lane has no dependency to
    install: this has to run on whatever machine the batch is started from.
    """
    request = {
        "model": DEEPSEEK_MODEL,
        "max_tokens": int(os.environ.get("DS_MAX_TOKENS", str(max_tokens))),
        "thinking": {"type": "adaptive", "display": "omitted"},
        "system": [
            {"type": "text", "text": prefix, "cache_control": {"type": "ephemeral"}}
        ],
        "messages": [
            {"role": "user", "content": f"{rules}\n\n```rust\n{body_text}\n```"}
        ],
    }
    out = subprocess.run(
        [
            "curl", "-sS", "--max-time", "900",
            "https://api.deepseek.com/anthropic/v1/messages?beta=true",
            "-H", f"x-api-key: {deepseek_key()}",
            "-H", "anthropic-version: 2023-06-01",
            "-H", "content-type: application/json",
            "--data-binary", "@-",
        ],
        input=json.dumps(request),
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    answer = json.loads(out)
    if "error" in answer:
        raise RuntimeError(json.dumps(answer["error"])[:200])
    return "".join(b.get("text", "") for b in answer["content"] if b["type"] == "text")


# ----------------------------------------------------------------- Gemini


def transcript(db: pathlib.Path):
    """`(steps, bytes sent)` for one `agy` session.

    The second number is a **prefix sum**, not the transcript's size: an
    agentic step is handed everything before it, so a session of n steps
    sends the transcript n times over. That is the shape of the cost and it
    is why this is measured rather than estimated — an earlier unmeasured
    fan-out of 412 sessions spent an estimated 2.2 billion input tokens,
    almost all of it in a tail of runs that never converged.
    """
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    sizes = [
        r[0] or 0
        for r in con.execute(
            "select length(step_payload)+coalesce(length(metadata),0) "
            "from steps order by idx"
        )
    ]
    con.close()
    running = sent = 0
    for size in sizes:
        sent += running
        running += size
    return len(sizes), sent


def run_gemini(prompt: str, minutes: int):
    """One `agy` session, and what it cost.

    Returns `(completed_process, seconds, steps, bytes_sent)`, with the last
    two `None` when no new transcript appeared — a session that left none is
    a session that never started, and saying so beats reporting a zero.
    """
    before = {p.name for p in GEMINI_CONVERSATIONS.glob("*.db")}
    started = time.time()
    proc = subprocess.run(
        [
            "agy", "-p", prompt, "--model", GEMINI_MODEL,
            "--print-timeout", f"{minutes}m", "--dangerously-skip-permissions",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    seconds = time.time() - started
    fresh = sorted(
        (p for p in GEMINI_CONVERSATIONS.glob("*.db") if p.name not in before),
        key=lambda f: f.stat().st_mtime,
    )
    steps, sent = transcript(fresh[-1]) if fresh else (None, None)
    return proc, seconds, steps, sent


def report_gemini(plural, singular, count, proc, seconds, steps, sent, answer_path):
    """The three lines every `agy` run prints, so both lanes say it the same.

    The minutes come first because they are the scarce thing on this lane:
    the subscription costs no money and refreshes a time quota instead.
    """
    print(f"{count} {plural}, {seconds / 60:.1f} min, Exit {proc.returncode}")
    if steps is not None:
        print(
            f"  {steps} Schritte, ~{sent / 4 / 1e6:.0f} Mio. Eingabe-Token, "
            f"{steps / max(count, 1):.1f} Schritte/{singular}"
        )
    print(f"  Antwort in {answer_path}")
