//! Nova Pentacle — {4} — Artifact
//! Oracle: {3}, {T}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.
//! Set: ME3 #200 — Masters Edition III | Scryfall ID: 0e627164-da4e-4860-b516-7304fc41161e | Oracle ID: db121503-8a34-498a-829c-72c33798369b
// PARTIAL — the whole ability is off the card: the redirect and its
// opponent-chosen recipient are not expressible, see the NOT SUPPORTED line
// above `abilities`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOVA_PENTACLE,
    oracle_id = "db121503-8a34-498a-829c-72c33798369b",
    scryfall_id = "0e627164-da4e-4860-b516-7304fc41161e",
    faces = &[face!(
        name = "Nova Pentacle",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no effect redirects damage from a chosen source to you onto a \
         creature, and no mechanism lets an opponent choose the recipient: \
         `Effect::RedirectNextFromChosenSource` goes the other way (a chosen \
         source's damage to a target creature is dealt to this ability's \
         controller)"
    ),
    // NOT SUPPORTED: "{3}, {T}: The next time a source of your choice would
    // deal damage to you this turn, that damage is dealt to target creature
    // of an opponent's choice instead." — the chosen-source shield exists
    // (`Effect::RedirectNextFromChosenSource`, Jade Monolith) but only in the
    // opposite direction: it shields a creature and moves the damage to the
    // ability's controller, and its `target` names the creature the damage
    // would have been dealt to, not a destination. Nothing here redirects
    // damage dealt to a player onto a permanent, and "of an opponent's
    // choice" is a chooser no `TargetSpec` or engine question carries: the
    // ability's target would be picked by its activator, where the printing
    // hands that choice to an opponent.
);
