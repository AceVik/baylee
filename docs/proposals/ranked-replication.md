# A second engine for ranked duels

## What runs today

Central reserves one verified gateway and binds one engine game to the match.
Both clients keep their home lobby connection. They open a separate game
connection to the assigned host, which retries delivery of the outcome. A
repeated identical result settles once. Another gateway cannot take over that
assignment. An interrupted match without a confirmed outcome can be voided.
[Ranked protocol](../protocol.md#gateway-hosted-ranked-duels)

The game host can replay the private starting preset and the ordered inputs
against a matching engine build. Its 64-bit state fingerprint is useful for
detecting divergence. Exact replay also checks for a terminal state. A result
verifier must read the winner from that reconstructed terminal state. The
ordinary record contains both deck orders and hidden actions and ships in
batches, normally at 32 KiB or after 30 seconds. It cannot promise recovery of
every move a player has already seen.
[Game record](../../crates/baylee-gamehost/src/record/mod.rs) ·
[Engine recording policy](../../crates/baylee-engine-server/src/lib.rs)

The Commander regression builds two independent game-host sessions from the
same seed and preset with two human seats. After each of 841 decisions it checks
both engine fingerprints against the archived reference and compares each
seat's outgoing messages. One session pauses
for 16 inputs and then catches up in order. Both engines identify seat 0 as
the winner on turn 29. A different legal opening mulligan changes the
fingerprint. Separate runs on Linux x86-64 and macOS ARM64 matched the same
archived sequence. Each run used two sessions in one process. Automatic
handover between network hosts still needs a separate end-to-end test.
[Twin session regression](../../crates/baylee-gamehost/src/session/tests/twins.rs)

## Independent replay

Reserve a second slot on an independently operated, verified gateway with the
same engine, rules configuration and card registry. Players' home gateways can
fill these roles when eligible. Two processes sharing a machine, database or
operator provide less independence. In a small pool, keep single-host matches
available and record which matches obtained independent verification.

Both engines receive one frozen setup, including the same seed, and the same
ordered, seat-authenticated decisions. Record clock expirations, concessions
and accepted reconnect decisions once, at the authority for the match. A
second local wall clock must never decide a different result. Neither players
nor spectators should receive the private transcript. Use short-lived,
match-scoped access for the verifier with explicit retention limits.

Have the second engine report the last input it applied and its independently
computed outcome. Start in observation mode to measure divergence, lag and cost.
Before using verification for points, bind the ordered inputs to authenticated
seat submissions independently of the primary, and use a reviewed cryptographic
digest for the committed input history. Two engines agreeing on a shared bug
cannot establish that a rule is correct. On disagreement, hold the result for
independent replay. Award Elo once, from Central.

## Taking over a live table

1. Central assigns a primary, a standby and an increasing generation number.
   Each accepted decision carries a stable command ID and expected position.
   Retries return the original decision's acknowledgement. Both engines apply
   each committed input once in sequence.
2. Before either seat sees the effects of a move that must survive failover,
   commit it to a durable log outside the primary's failure domain. Record the
   agreed private position and clock state. Synchronous replication adds a
   network round trip. Measure it at peak load and across real hosting regions.
3. After a primary failure, Central revokes its generation, checks the standby
   through the last acknowledged input and promotes it atomically. Both clients
   obtain new seat tickets for the replacement host, accept any outstanding
   host terms and receive their own projected state. They retain their
   lobby connections. An unacknowledged action can be retried under its original
   command ID. Close and settlement from the old primary must be rejected.
4. If a partition prevents a safe ownership decision, pause the game and its
   decision clocks. If the standby is missing or behind, recover the committed
   suffix before accepting new moves. When recovery is impossible, release
   the seats without charging a player for the outage. Expose single-host
   availability honestly when a second verified slot is unavailable.

Tower already exercises a generation-bound durable prefix and recovery.
Ranked needs that boundary for two human seats, independent result reporting,
host revocation and the client handover. Before enabling automatic failover,
test loss of each host and Central, delayed messages from the old primary,
duplicate/out-of-order actions, mismatched builds, divergent state, hidden
information after reconnect and loss immediately before and after settlement.
[Tower recovery tests](../../crates/baylee-engine-server/src/tower/durable/tests/recovery.rs)
