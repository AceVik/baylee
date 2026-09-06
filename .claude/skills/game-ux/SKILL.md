---
name: game-ux
description: Game interface and interaction design — readability at a glance, feedback, irreversible actions, accessibility and multi-input layout. Use when adding or changing anything a player sees or clicks: HUD, prompts, hand, board, lobby, deckbuilder, settings.
---

# Game UX and UI

Current practice, checked 2026-09-06. The consensus has moved away from
decorative polish toward *reading a situation quickly and acting confidently*.
Interface quality is not cosmetic: a substantial share of players abandon games
over the interface rather than the gameplay.

## Read at a glance

The player must be able to answer, without hunting: **what changed, what is
urgent, and what can I do next.** Encode state in form as well as in number — a
colour, a pill, a border light — so the answer survives a glance.

Keep separate claims visually separate. In this client, a *keyword* (what a card
is) is drawn as a steady sheath, while *activatable* (what you could do) is a
warm light travelling round the border. Two different claims must not read as
the same light — collapsing them is how a player learns to distrust the display.

## Feedback, and the cost of a wrong tap

Every action needs immediate confirmation that it registered. And where an
action cannot be undone, the interface owes the player a second step:

- **Two-stage arming.** There is no undo in this engine, so a first tap arms and
  a second sends; `Esc` takes it back. Every reader re-resolves against the
  *current* legal actions, so an option the engine has withdrawn disarms instead
  of firing.
- **Know the one exemption.** Mana abilities stay a single tap, because floating
  mana is the cheap mistake. Exemptions should be principled and few.
- **Never let a stale frame act.** A menu drawn a frame ago must rebuild from
  the current legal actions when pressed, and send by position — not send a
  remembered action.

## Offer only what is real, and say what is only an offer

Three states, not two, are usually needed: what the engine says is legal, what
*this client* is offering to arrange on your behalf (tapping lands to make a
spell castable), and what a permanent could do. Conflating "the game allows
this" with "the client will try this" makes the client's helpfulness look like
a rules bug when it fails.

Corollary: an interface that offers something the engine will refuse is worse
than one that offers less. Mirror the server's validation exactly — *if the
button is live, the action succeeds.* This repo states that invariant for the
deck builder and it is the right shape everywhere.

## Accessibility is structural

- Contrast to WCAG AA: **4.5:1** for normal text, **3:1** for large text.
  Assert it in tests where you can; a one-sided "is it dark enough" check
  already let a bug through here.
- Reinforce icon with text; never let colour alone carry meaning.
- Scalable text, remappable input, and honour reduced-motion.
- Localisation as a *typed* surface: this codebase makes a missing translation a
  compilation error rather than an English fallback, so half a screen can never
  render in the wrong language.

## Design for every input from the start

Touch, controller, keyboard and TV distance are not adjustments made afterwards.
Concretely here: `Metrics::of(width)` picks a phone/tablet/desktop frame and
every size derives from it, phones get 44-logical-pixel targets, and canvas text
entry uses a real invisible `<input>` so the phone keyboard, IME, autofill and
paste all work. Rolling your own text input on a canvas loses all four.

## Where the logic goes

Keep decisions in a renderer-free layer (`baylee-client-core`) and let the
renderer draw. That is what makes the interface testable headlessly — including
the interaction state machine, the layout and the lobby flow — and it is what
catches the classic failure where the engine offers something and the client has
no button for it.
