---
name: game-design
description: Systems and card-game design judgement — depth versus complexity, state machines and priority, and the constraints a rules engine imposes. Use when designing a mechanic, weighing a rules change, evaluating AI behaviour or format rules, or deciding how much of a rule belongs in the engine.
---

# Game and systems design

## Depth is not complexity

The easiest trap in card-game design is mistaking the two. **Complexity** is
more rules, more clauses, more exceptions. **Depth** is more meaningful
decisions with the same rules. A mechanic that adds a paragraph of text and one
new decision per game is a bad trade; one that adds a sentence and changes how
every existing card is evaluated is a good one.

Applied here: this project implements an existing rules set rather than
inventing one, so the design work is mostly about *which* rules the engine
expresses and how honestly it refuses what it cannot.

## The honest-stub rule, and why it generalises

The card transcoder may only emit a card it understood **in full**: one unread
clause and the card stays an explicit `Coverage::Unimplemented`. This is a
design principle, not a coding one. A system that silently half-implements
something is worse than one that implements nothing, because the deckbuilder
offers `Implemented` cards as playable — a wrong "yes" costs a player a game,
a truthful "no" costs them a card.

Generalise it: **make partial support unrepresentable or explicit, never
implicit.**

## Read a blocker by what the language cannot *say*

When a feature looks blocked, the instinct is to assume a missing subsystem.
Usually the mechanism exists and the vocabulary does not. The `Pump` case here
is the canonical example: the layer machinery, the filters and the continuous
effects were all present; what was missing was one variant that could say "the
target" without overloading a filter to mean it. That one variant moved the
transcoder from 1886 to 2545 scripts.

So: before designing a subsystem, write the sentence you cannot express today
and ask which single word in it has no representation.

## State machines, priority, and where a rule lives

A card game is a state machine with strict phase logic, and the useful
discipline is that **the engine advances only through enumerated choices**. This
engine publishes a `Pending` listing the legal actions and validates the answer
against that same enumeration — there is no "cast this spell" method. The
consequences are worth designing toward:

- A client cannot invent an option, so a client bug cannot become a rules bug.
- Anything a player may choose must be *offered*, which forces the designer to
  enumerate the cases rather than hand-wave them.
- A probe that decides "is this legal" and a wizard that decides "how would it
  be cast" must give the same answer. When they disagree, the game offers a
  spell it then refuses — a real defect found in this codebase.

## Loops, draws and progress

An endless loop is detected over a *rules-visible* signature, not over a state
hash that never repeats. The design question for any new counter is: **does it
represent progress?** A commander tax that only grows means a repeated cast is
not a loop at all, so it belongs in the loop signature. A cosmetic counter does
not. Getting this backwards turns a progressing game into a false draw.

## Designing for AI and for anti-cheat at the same time

The house AI takes `(&PlayerView, &Pending)` — exactly what a networked seat
receives — so it *cannot* reach hidden information even by mistake. Designing
the AI's interface as the player's interface is what makes a drivable AI seat,
a scripted test opponent and a future LLM opponent all safe by construction
rather than by remembering to be careful.
