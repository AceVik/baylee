---
name: mtg-rules
description: Magic's Comprehensive Rules as an engine implements them — which shape a printed sentence takes (state-based action, trigger, replacement effect), the layer system, the casting procedure, targeting and combat, and the citation discipline this repo enforces. Use when implementing or debugging a rule, reading a card's oracle text, deciding where a mechanic belongs in the engine, or writing a comment that cites a CR number.
---

# Magic's rules, for people writing an engine

Verified 2026-09-06 against the Comprehensive Rules. The CR is revised with
every set; a number that was right last year may have moved.

## Rule zero: look the number up, do not recall it

**Never write a CR citation from memory.** In one audit of eight freshly
recalled citations in this repo, four were wrong. A wrong number is worse
than none: it reads as verified, it survives review, and the next person
builds on it.

One web search per number, or `grep -rn "CR 613" crates/ docs/` for one this
repo has already used and tested. The repo is not the CR — a number here is
evidence somebody looked once, not proof they were right — but a number with
a passing test behind it is far better than a number from memory.

The numbers this codebase leans on most, by frequency, are `305.6` (intrinsic
mana from a basic land type), `903.9a`, `115.4`, `903.8`, `508.1a`, `613.1`,
`605.1`, `601.2c`. Those have tests.

## Which shape is this sentence?

Most implementation mistakes are not "the rule was coded wrong" but "the rule
was coded as the wrong *kind* of thing". A printed sentence is one of four,
and they run at different times with different interruptibility:

| Shape | Runs | Uses the stack | Can be responded to |
|---|---|---|---|
| **State-based action** (CR 704) | Whenever a player would get priority, as a fixpoint | No | No |
| **Turn-based action** (CR 703) | At the start of a step or phase | No | No |
| **Triggered ability** ("when", "whenever", "at") | Put on the stack next time a player would get priority | Yes | Yes |
| **Replacement effect** ("instead", "as … enters", "if … would") | Continuously, modifying an event before it happens | No | No |

The tell is usually the wording. "Instead" and "would" mean replacement.
"When/whenever/at" means trigger. A rule with no trigger word that just
*is true* is state-based.

Commander shows both halves of one mechanic split across two shapes, which is
why it is the example worth remembering: **CR 903.9a** — a commander in a
graveyard or exile may go to the command zone — is a **state-based action**,
because it is checked whenever SBAs are. **CR 903.9b** — the same move from
hand or library — is a **replacement effect**, because there is no moment
afterwards at which to notice. Implementing 903.9b as an SBA would silently
never fire for a milled commander.

## Layers (CR 613)

Continuous effects do not apply in the order they were created. They apply in
seven layers, always in this order:

1. Copy effects
2. Control-changing effects
3. Text-changing effects
4. Type-changing effects
5. Color-changing effects
6. Ability adding and removing
7. Power and toughness

Layer 7 has sublayers, applied in their own order: **7a** characteristic-
defining abilities, **7b** effects that *set* P/T to a value, **7c** effects
that *modify* P/T (+N/+N), **7d** counters. That ordering is why a creature
animated to 4/4 and then given +1/+1 is a 5/5 and not the other way round,
and why a `-1/-1` counter is applied after a pump.

One printed sentence is often several layer entries. "Creatures you control
get +1/+1 and have flying" is layer 6 *and* layer 7c, and they must be applied
in layer order, not in reading order.

Within a layer, timestamps decide, and dependency (CR 613.8) can reorder
before timestamps do.

## Casting is a procedure, and it is reversible

CR 601.2 is a sequence: announce, choose modes and targets, determine total
cost, activate mana abilities, then pay. Two consequences that catch engines:

- **Cost changes have an order.** Cost *increases* (a commander's tax, a
  ward-style tax) and cost *reductions* (convoke, delve, affinity) do not
  commute with each other in general; the CR fixes the order, increases before
  reductions. An increase applies to every way of casting the card —
  the printed cost, an alternative cost, and each mode alike (CR 601.2f).
- **A cast that cannot be paid is reversed in full** (CR 601.2h). Not
  partially. If a spell taps creatures for convoke and then finds the mana
  short, the creatures must be untapped again and the spell must return to
  hand exactly as it was. This repo shipped the half-reversal for as long as
  convoke was unreachable: the taps were spent before the mana was.

The mirror of that rule is an offer-and-payment one: **whatever the engine
counts when deciding a spell is castable must be the same count it uses when
paying.** A spell offered as castable and then refused mid-wizard leaves the
player tapped out with nothing to show for it. Prefer one function that both
call over two that agree today.

## Targets

- Targets are **distinct** (CR 601.2c). Naming the same object twice for a
  two-target spell is an illegal answer, not a double hit.
- A teammate's permanents are **legal** targets (CR 115.4). The engine must
  offer them; not shooting your own side is the *agent's* judgement, not the
  engine's legality check.
- "Any number of target …" is min 0. A legal answer of *no* targets is a
  spell that does nothing — and, for anything automated, a spell that changes
  no state and so gets cast again on the next priority. Forever.
- Targets are checked twice: on announcement and again on resolution. A spell
  whose every target became illegal does not resolve.

## Combat

The steps are: beginning of combat, declare attackers, declare blockers,
combat damage, end of combat. First strike inserts an extra damage step.

**Which creature may block which attacker is a pairing question**, not two
lists (CR 509.1a). Flying, menace, protection and "can't be blocked by" all
live in the pairing. An engine should answer it — enumerate the legal
pairings — rather than hand a client two flat lists and hope. Likewise which
defender may be attacked (CR 508.1a) is the engine's answer, because a
planeswalker's attackability is not a property of the attacker.

Damage maths that heuristics routinely get wrong:

- **Lethal damage counts damage already marked.** A 4/4 with 3 damage on it
  dies to a 1/1.
- **Deathtouch** (CR 702.2b): any nonzero amount is lethal. **Indestructible**
  (CR 702.12b) answers it.
- **First strike** is the one that turns a trade into a free kill: if exactly
  one side strikes first *and kills*, the other never deals its damage at all.
  Two `>=` comparisons cannot express this; a single "who dies" function can.
- **Trample** (CR 702.19b): a chump block does not stop the excess.

## Mana

- A **mana ability** does not use the stack and cannot be responded to
  (CR 605.1). That is the exception, not the rule, which is why marking an
  ordinary ability as one is a silent rules bug rather than a visible error.
- A basic land type grants an **intrinsic** mana ability (CR 305.6). Taiga
  prints nothing but reminder text; the ability comes off the type line. This
  is the most-cited rule in this repo for a reason — any land handling that
  reads printed text only will get dual lands wrong.

## Where to look in this repo

- `docs/engine-internals.md` is normative on layers, events, SBAs and loop
  detection as implemented.
- `docs/card-dsl.md` is the card-authoring contract.
- `docs/mechanics-roadmap.md` §A lists what is already supported — check it
  before scheduling something as new work.
- Engine tests live inline as `crates/baylee-engine/src/engine/*_tests.rs`.
  A new mechanic gets one test that *plays* it, not one that inspects state
  the mechanic happens to write.

## Two habits that have paid off here

**Assert through the offer, not through the counter.** A test that reads back
the field a rule wrote will pass while the rule is never consulted. A test
that asks "is this card castable now?" found a commander tax that was recorded
for three months and charged never.

**When a rule seems absent, look for what the DSL cannot *say*.** The
mechanism is usually already there and missing one variant. Reading a coverage
report as a list of missing subsystems schedules work that does not exist.
