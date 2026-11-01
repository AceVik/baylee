//! Nature's Blessing — {2}{G}{W} — Enchantment
//! Oracle: {G}{W}, Discard a card: Put a +1/+1 counter on target creature or that creature gains banding, first strike, or trample. (This effect lasts indefinitely. Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding a player controls are blocking or being blocked by a creature, that player divides that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: ME2 #196 — Masters Edition II | Scryfall ID: a5a1c3c6-4923-40d3-ad38-e05b9d93c34e | Oracle ID: ae0f65b4-3814-45fe-a869-8585f6869278
// IMPLEMENTED — {G}{W}, Discard a card activated ability putting a +1/+1 counter on target creature or granting it banding, first strike, or trample indefinitely.

use baylee_cards_dsl::prelude::*;

static CHOICES: &[Effect] = &[
    Effect::AddCounter {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(1),
    },
    Effect::ModifyTargets {
        modifier: Modifier::AddKeyword(KeywordSet::BANDING),
        duration: Duration::Indefinitely,
    },
    Effect::ModifyTargets {
        modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
        duration: Duration::Indefinitely,
    },
    Effect::ModifyTargets {
        modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
        duration: Duration::Indefinitely,
    },
];

card!(
    index = index::NATURE_S_BLESSING,
    oracle_id = "ae0f65b4-3814-45fe-a869-8585f6869278",
    scryfall_id = "a5a1c3c6-4923-40d3-ad38-e05b9d93c34e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nature's Blessing",
        mana_cost = mana!("{2}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{G}{W}", Discard(&Filter::Any)),
        &[Effect::ChooseOne { choices: CHOICES }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
