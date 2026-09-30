# What a question holds an answer to

A survey of every refusal `Engine::apply` makes, per kind of question, taken
on 2026-09-29 against `origin/main` a396e52e, and what became of each one.
The contract since: **`apply` refuses an answer only for a reason the
question states in machine-readable form, and takes every answer inside
what it states.** `Pending::answer_fault(&PlayerAction) -> Option<AnswerFault>`
is that statement as one pure function: `apply` runs it first and refuses
with its reason (`EngineError::IllegalAction(fault.reason())`, or
`MismatchedAction` for an answer of the wrong kind), and a client, the house
AI and a trained agent can run it before they send. Only the seat comes
before it: a seat the question does not ask is refused with
`MismatchedAction` whatever it answered, since a search's options are
cards in a hidden library and a refusal naming a fault in them would tell
a bystander which of its guesses were there.

The defect that started it: the trained AI's fuzzer (2000 games, main
f7390913) answers inside what `Pending` states and was refused 38 crew
answers (`ChooseCards`, `min` 1, `CostCrew { power: 2 }`, one creature of
power 1) and 11 blocker declarations (one creature against a menace
attacker). Convoke, `ChooseNumber` and `ChoosePlayer` refusals were fixed
before this survey and stay at zero: the refusal sweep below, whose driver
answers the same way, finds none on the whole house-deck grid.

## Refusals no field of the question stated

| question | refusal on a396e52e | rule | stated now by |
| --- | --- | --- | --- |
| `ChooseCards` (`CostCrew { power }`) | "not enough power to crew": the chosen creatures' total power below N | CR 702.122a | `ChooseCards.total: Some(CardTotal { of: Power, weights, at_least: Some(N), at_most: None })`, fault `TotalTooLow` |
| `ChooseBlockers` | "menace requires two blockers": exactly one blocker on a menace attacker | CR 702.111b | `ChooseBlockers.bounds: Vec<AttackerBound>`, menace `2..=u32::MAX`, faults `TooFewBlockers`/`TooManyBlockers` |
| `Mulligan` | a take once the hand would open with zero cards | CR 103.5 | `Mulligan.can_take: bool`, fault `NoFurtherMulligan` |
| `ChooseBlockers` | "duplicate blocker": one creature named against two attackers | CR 509.1a ("chooses one creature for it to block") | `BlockOption`'s documented shape (each blocker at most once), fault `Repeated` |

Three refusals came *after* an answer the question offered had been taken,
from a continuation that could not go on. Each now reverses what it began
instead (CR 732.1: an action that cannot be completed is reversed), and
the answer is taken:

| question | answer | was | now |
| --- | --- | --- | --- |
| `ChooseCards` (pitch cost) | the offered card | refused when the cast could then not be paid | the cast is reversed (`continue_cast_wizard`) |
| `ChoosePlayer` (whose graveyard, in an activation) | an offered player | refused when `start_activation` then failed | the activation is reversed (`reverse_activation`) |
| `YesNo` (Phyrexian mana: pay life?) | yes or no | refused when `start_activation` then failed | the activation is reversed |

## Refusals the question states (unchanged in substance)

Membership, counts and ranges were stated all along; they are now one
checker instead of a check per arm, with one reason per kind of fault:
`NotOffered` (a card, pairing, mode, pile, colour, subtype or player the
question did not list; a priority press not in `legal`), `Repeated`,
`TooFew`/`TooMany` (`min`/`max`, `count`), `OutOfRange` (`ChooseNumber`),
`Misarranged` (`Arrange`'s piles, `arrange::arrangement_fault`), `WrongKind`.
The per-arm checks behind the gate (`declare_attackers`' "creature cannot
attack", "invalid defender", "duplicate attacker"; `declare_blockers`' "no
such attacker", "creature cannot block"; "no such cast mode") stay as
defence in depth and are unreachable past it.

## Refusals that remain, and why

- **Another seat's answer, or any answer once the game is over.** The
  question names the seat it asks (`Pending::asked`); `GameOver` takes none.
- **A card not in the answering seat's hand** (`MulliganBottom`,
  `DiscardChoice`: "card not in hand"). The options are the seat's own
  hand, which its view lists in full; the question carries only `count`.
- **A card name the pool does not print** (`ChooseCardName`: "not a card
  name"). The options are the card pool (CR 201.4), which every client and
  agent of the same build has; a list would send thousands of names a frame.
- **A target batch whose series changed** (`ChooseTargetBatch`: "target
  series changed"). Its bound is `PlayerView.targeting.batch_count`, stated
  in the chooser's view rather than in `Pending`.
- **A priority press the offer listed and the payment then refuses**
  (`CastFailure`, a suspend cost). `legal` promises the press can be paid;
  a refusal here is an offer/apply disagreement, a defect held by
  `offer_tests`, the dual-land test in `refusal_tests` and the sweep's
  `refused_clean` finding, not a constraint to state.
- **A draw offer without priority, or with nobody left** (`OfferDraw`). It
  answers no question; it is a side action a seat may take while it holds
  priority.
- **Plan guards** ("no subtype choice pending", "no card-name choice
  pending", the fallback `MismatchedAction` in `apply_inner`): the question
  and the engine's plan out of step, which no answer can cause.

## Requirements (CR 508.1d, 509.1c)

A requirement ("attacks each combat if able", "must be blocked if able")
makes a declaration illegal when it obeys fewer requirements than the most
that could be obeyed without breaking a restriction. **The engine implements
no requirement today** (the DSL has restrictions only: `CANT_ATTACK`,
`CANT_BLOCK`, `UNBLOCKABLE`, `CantBeBlockedBy`), so no such refusal exists
and none is stated. The empty declaration, which `timeout_answer` gives, is
therefore always accepted.

When one is implemented it is stated the same way, and stays checkable
from the question alone: the question lists the requirements
(`requirements: Vec<Requirement { creature, attacker_or_defender }>`) and
the maximum the engine computed (`most_obeyed: u32`, the search being the
engine's), and `answer_fault` counts the requirements a declaration obeys
against that number (`AnswerFault::FewerRequirementsObeyed`). The sweep's
driver then samples declarations below and at the maximum like any other
bound.

## Where it is held

- Rules: `crew_states_its_total_power_and_apply_refuses_only_what_it_states`
  (`card_tests/artifacts.rs`), `menace_states_its_blocker_bound_and_apply_refuses_only_what_it_states`
  (`combat_choice_tests.rs`), `no_mulligan_is_taken_past_a_hand_of_zero`
  (`house_rules_tests.rs`), the `answer_fault` unit tests in
  `choice.rs` (`fit_to_options_tests`).
- The sweep (`refusal_tests.rs`): every sampled answer is held to the
  question both ways; the bounds deck plays sixteen mirror games where crew
  and menace are asked; the same games with the driver told each question
  less its bounds show the sweep naming both refusals.
- Consumers: `baylee-ai` answers crew by `total` (`policy::reach_total`)
  and holds every profile's blocks to `bounds` (`combat::keep_bounds`);
  `baylee-client-core` sends no answer `answer_fault` faults
  (`Interaction::confirm`, `can_confirm`, `answer_fault`).
- Wire: `docs/protocol.md` §"What a question holds an answer to".
