//! Pyramids — {6} — Artifact
//! Oracle: {2}: Choose one —
//! Oracle: • Destroy target Aura attached to a land.
//! Oracle: • The next time target land would be destroyed this turn, remove all damage marked on it instead.
//! Set: ARN #67 — Arabian Nights | Scryfall ID: d2e9decf-47b7-44e0-b380-8055b6011021 | Oracle ID: da2f0d16-3cb4-492d-8535-52a31dbae95e
// PARTIAL — the Charm and both of its modes are off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PYRAMIDS,
    oracle_id = "da2f0d16-3cb4-492d-8535-52a31dbae95e",
    scryfall_id = "d2e9decf-47b7-44e0-b380-8055b6011021",
    faces = &[face!(
        name = "Pyramids",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no modal activated ability exists, no filter names an Aura by what it \
         is attached to, and no destruction replacement or remove-damage \
         effect exists"
    ),
    // NOT SUPPORTED: "{2}: Choose one —" — `AbilityDef` has `ModalSpell` and
    // `ModalTriggered` but no modal activated ability, so one ability with two
    // modes cannot be stated; two separate `{2}` abilities would be two
    // abilities to copy, counter or activate, which is not the Charm.
    // NOT SUPPORTED: "• Destroy target Aura attached to a land." — the destroy
    // is sayable (`Effect::Destroy`), but no `Filter` names an Aura by its
    // host: `Filter::AttachedToBySource` reads from the ability's source
    // (Pyramids, attached to nothing) and `Filter::IsAttached` says only that
    // the Aura is attached, not to a land. Targeting a bare Aura and letting
    // an `IfTargetMatches` fizzle would offer targets the printed card does
    // not.
    // NOT SUPPORTED: "• The next time target land would be destroyed this
    // turn, remove all damage marked on it instead." — `Effect::Regenerate`
    // is the closest shield, but a regeneration shield also taps the
    // permanent and removes it from combat (CR 701.19a), which the printed
    // sentence does not; no effect removes marked damage without those riders,
    // and no replacement rule hooks destruction.
);
