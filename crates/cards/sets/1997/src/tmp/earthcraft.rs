//! Earthcraft — {1}{G} — Enchantment
//! Oracle: Tap an untapped creature you control: Untap target basic land.
//! Set: TMP #222 — Tempest | Scryfall ID: 9dda7531-82a1-4f49-8858-601ddbc6e2bc | Oracle ID: 50aa7aff-1f01-4224-9a83-01f74d703ec2
// IMPLEMENTED — the pool's first `CostPart::TapOther`: a cost paid by tapping
// a permanent that is not the source. Which creature is asked as the ability
// is activated, by `engine::cost_wizard`, and the answer is the one of the
// three asking costs whose card survives being named.
//
// Two things the card does not say and the rules do. The creature must be
// untapped (CR 118.3: a permanent already tapped cannot be tapped to pay a
// cost), and it may be summoning sick — CR 302.6 restricts a creature's own
// `{T}` ability and says nothing about it paying for somebody else's, so a
// Bird cast this turn can already work the land.
//
// The ability is not a mana ability (CR 605.1): it untaps a land rather than
// adding mana, so it uses the stack and can be responded to, even though
// every deck that plays it plays it to make mana.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARTHCRAFT,
    oracle_id = "50aa7aff-1f01-4224-9a83-01f74d703ec2",
    scryfall_id = "9dda7531-82a1-4f49-8858-601ddbc6e2bc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Earthcraft",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(TapOther(&Filter::YOUR_CREATURE)),
        &[Effect::UntapTarget],
        target = Some(TargetSpec::Object(&Filter::BASIC_LAND))
    )],
);
