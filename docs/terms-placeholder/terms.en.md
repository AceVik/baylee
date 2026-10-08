<!-- version: placeholder-1 -->
<!-- updated: 2026-10-08 -->

# PLACEHOLDER — not legal text

**This file is a placeholder for testing the terms sheet in more than one
language (WG-1). It is not legal text, it is not the terms of any gateway,
and no gateway should serve it to players.** The owner writes the real
terms; they live on the server only and never in this repository.

Nothing reads this directory unless an operator points `BAYLEE_TERMS_PATH`
at it. No deploy script does.

## What it shows

A gateway whose `BAYLEE_TERMS_PATH` names a directory serves the
`terms.<lang>.md` in the client's language, and this English one to every
language the directory lacks. Every file names the same version on its
first line, so accepting in one language accepts them all.

- Switch the client's language while the sheet is up: the text follows.
- Ask for a language not here: this text comes back.

---

A last paragraph, long enough to scroll on a phone held sideways, so the
sheet's Accept button can be seen to wait until the end has been reached.
See <https://example.invalid/terms> for nothing at all.
