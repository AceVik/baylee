---
name: mmo-architecture
description: Designing a game server that has to hold many simultaneous sessions — where to put authority, what a process boundary buys, how hidden information and anti-cheat stay structural, and which seams matter when a single-table game grows into a persistent one. Use when adding a service, choosing where state lives, deciding what a client is told, or judging whether a seam will still hold with thousands of concurrent tables.
---

# Architecture for a game that will have many players at once

The scaling question is almost never "can this go faster". It is "what does
this piece of state belong to, and who is allowed to say it changed". Get
that wrong and no amount of throughput helps; get it right and most scaling
is running more of what you already have.

## The authority rule

**Exactly one process decides the rules, and it is never the client.** Every
other component routes, stores, or renders. This sounds obvious and is
routinely violated by convenience: a client that computes what it is allowed
to do, a lobby service that peeks at a game to answer a listing, a cache that
becomes the truth because it is faster to read.

The test for a component: could it *lie* about the game if it were
compromised? If yes, it holds authority and must be counted as part of the
trusted core. This repo keeps that line visible by making the gateway link
neither the rules kernel nor the session layer — a dependency that cannot be
added by accident.

## One process per session is a boundary you can spend

Running each game in its own process buys three things at once, and they are
usually bought separately and expensively:

- **A panic boundary.** A crash takes one table down, not the fleet. That
  removes the defensive `catch_unwind` scaffolding around the rules entirely.
- **A scheduling unit.** Sessions are what you place on machines, drain, and
  bill for. A process is already the operating system's unit for that.
- **A memory ceiling.** One table cannot exhaust another's.

The cost is a supervisor, a handshake and a per-session secret. A *supervisor
that only spawns* — this repo's agent links the wire protocol and
`std::process` and nothing else — is small enough to be obviously correct,
and it is the piece that later becomes "run games on other machines".

Do not reach for this per *player*; reach for it per *unit of shared state
with a lifetime*. A table, a raid instance, a match. Players come and go
inside it.

## Hidden information must be unrepresentable, not filtered

The dangerous shape is a full state object with a redaction pass on the way
out, because the redaction is a place a mistake can be made once and made
everywhere. The safe shape is a *projection type* that has no field for the
secret: another player's hand is a count, so no code path can leak the cards
because there is nowhere to put them.

Two consequences worth planning for:

- **The projection needs its own version number**, asserted on both sides, so
  an old client refuses a server it cannot render instead of drawing nonsense.
- **Anything the client cannot compute must be projected**, or it will guess.
  A client that cannot run the effect system needs post-effect values shipped
  to it. The failure mode is subtle: an offer and a projection that disagree
  is a move the interface invites and the server refuses.

The bug this rule catches late is *shared* data: a table-wide lookup table
sent to everyone is the union of everyone's private inputs. The fix is holes
rather than a shorter list, because the index is what everything else points
at, plus a rule for earning entries.

## Anti-cheat is a shape, not a subsystem

If rankings exist, cheating pays, and a "server validates everything" claim
needs a structure behind it:

- **Enumerate legal moves, don't validate free-form ones.** When the server
  publishes the set of legal actions and accepts only an answer from that
  set, the client cannot name a move that was never offered. Validation
  becomes a set membership test rather than a growing pile of checks.
- **Type the boundary the automation crosses.** If a bot or an AI opponent
  takes the same input a networked player takes — literally the same
  projection type — it cannot see more even by mistake. That is stronger than
  a convention and free at runtime.
- **Determinism is the audit trail.** A seeded RNG, no iteration over
  unordered collections in decision paths, and no clock reads in the rules
  mean a game can be replayed from its inputs. That is how a disputed match
  is settled and how a desync is diagnosed.
- **The development back door has to be built to the same line.** A remote
  control that sees a seat's own view is a tool; one that reads state
  directly is a cheat waiting to be shipped. Gate it behind a compile-time
  feature so it is absent from a release binary, not merely off.

## Presence and authority over a chair are different questions

A player disconnecting is not a player leaving, and neither is a player being
replaced. Model them separately or the table stalls:

- A seat with **no connection** should be on no decision clock — nobody
  should lose to a question they never received.
- But "no clock" left alone means the whole table waits on someone who closed
  their laptop. So there are *two* timers: one for deciding, one for coming
  back, and they expire into different things — the first answers a question,
  the second hands the chair to an automated stand-in.
- **A held chair is not relabelled.** A seat that renamed itself after a
  thirty-second hiccup keeps saying so after the player returns. Carry
  "away" as its own flag beside "is automated".
- The **roster** is state too. If it travels in a one-shot payload, taking a
  chair over invalidates it for every other participant; something has to
  mark it stale.

## Anchor timeouts to questions, not to messages

A decision clock anchored to frames sent can be wound by an opponent holding
priority or by a reconnect storm. Anchor it to a counter that only advances
when a new decision is actually asked. The same reasoning applies to any
rate limit or idle timer that a hostile peer can influence.

## Reconnection is policy, and it needs a place to live

Transports know how to redial. What they cannot know is *whether to*, how
often, and when to give up — and if the abstraction over the transport hides
"is the link up", nothing above it can decide. Expose link state as a small
enum with a distinct **connecting** state (or the scheduler redials once per
frame for as long as a socket takes to open), and put the schedule in a
transport-free module so it is testable without a server to disconnect from.
Exponential backoff to a cap, a bounded number of attempts, then say so.

Give up into a *distinct* outcome. If "the server refused this one action"
and "the server is gone" arrive as the same error, a client that returns to
the lobby on the second will eject players on the first.

## Secrets: one per hop, none interchangeable

A supervisor token, a per-session token, and a per-seat token are three
different things and must not be substitutable. The tempting shortcut —
"the seat token is the seat number" — is defensible only inside a loopback
development harness, and then only if that harness binds to loopback *by
default*, because the failure is silent and total.

Also: a token handed out before the thing it names exists is a trap. Either
provision the session before answering, or make the client wait on an
observable state change rather than on a socket that will be accepted and
immediately closed.

## Push the list, but page it in a total order

A lobby, a friends list, a leaderboard: all of them are "a page of a
collection that changes". Two rules save a lot of grief.

- **Page in a fixed total order.** Paging an unordered collection — a hash
  map, an unsorted query — hands some rows out twice and never shows others.
  Sort by something stable, then by id as the tiebreak.
- **The subscription includes the query.** If a client's search and page
  offset are part of what it is subscribed to, changing either has to
  re-subscribe. And per-viewer fields ("is that me", "is that mine") mean the
  payload is rendered per socket, not broadcast.

Keep the polling path. A pushed feed that cannot be established must degrade
to a re-read, not to a blank screen.

## Where to put the database, and where not to

Two different kinds of state, two different answers:

- **Catalog / reference data** — item definitions, card text, localisation.
  Large, read-mostly, shared by everyone, and *optional to the rules*. This
  belongs in a real database with real indexes, behind its own service or
  crate so the dependency does not creep into anything that must stay
  portable. If it is genuinely optional, make missing it a degraded mode
  rather than a startup failure.
- **Session state** — what is happening at this table right now. This lives
  in the process that owns it. Writing it to a database per action is how a
  game server acquires a bottleneck it can never remove.

Between them sits **account state** — profiles, inventories, ratings — which
is durable, small per player, and written at session boundaries. That is a
database's job, but the write pattern is "at the end", not "per tick".

The staged migration this suggests is the right one: a file-backed store is
fine while the shapes are still moving; move it when the access pattern
(concurrent writers, queries you cannot do in memory) actually demands it,
and move one concern at a time.

## Seams to check against "many at once"

When adding anything, ask these five:

1. **What owns this state, and can two owners disagree?**
2. **Does it hold per-session memory that grows with players?** A per-table
   cache is fine; a global one keyed by player is a leak with a schedule.
3. **Is it on the path of every action, or only of session setup?** Setup
   costs are cheap to scale; per-action costs are the ones that need care.
4. **Would it still work if this component ran on another machine?** If it
   reads a local file, a local port, or a shared process's memory, it will
   not. That is the moment a default like "dial localhost" becomes wrong.
5. **What happens when it is not there?** Every component is sometimes
   missing. Decide between degraded mode and refusal explicitly.

## Avoid vendor lock-in until the stack is chosen

Every managed service that reaches into the code — a proprietary client
library in the rules, a schema only one engine can express, a queue whose
semantics you rely on — is a decision made early with the least information
you will ever have. Keep the interfaces at the seams generic (a store trait,
a transport trait, a config value rather than a hard-coded host) and let the
choice be made once there is a load to size it against.
