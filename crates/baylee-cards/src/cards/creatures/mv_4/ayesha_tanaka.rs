//! Ayesha Tanaka — {W}{W}{U}{U} — Legendary Creature — Human Artificer
//! Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: {T}: Counter target activated ability from an artifact source unless that ability's controller pays {W}. (Mana abilities can't be targeted.)
//! Set: CHR #73 — Chronicles | Scryfall ID: 8ce912d9-406b-4eba-97be-3bf1d425ee05 | Oracle ID: 35e4ed0e-8ce2-4729-a009-0d6be8f47663
// PARTIAL — banding is written; the counter ability is off the card.
// NOT SUPPORTED: "{T}: Counter target activated ability from an artifact
// source unless that ability's controller pays {W}. (Mana abilities can't be
// targeted.)" — `TargetSpec::AbilityOnStack(filter)` matches the ability
// object itself, whose characteristics are the blank base, and no filter
// reads the ability's source or whether it is an activated ability, so
// "from an artifact source" cannot be stated; an unrestricted
// `Effect::CounterTargetAbility` would also counter triggered abilities and
// abilities from every other source, which is stronger than the card
// prints. The `{W}` tax itself is sayable
// (`Effect::PlayerMayPayManaOr` with `PlayerRel::ControllerOfTarget`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AYESHA_TANAKA,
    oracle_id = "35e4ed0e-8ce2-4729-a009-0d6be8f47663",
    scryfall_id = "8ce912d9-406b-4eba-97be-3bf1d425ee05",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::BANDING,
    coverage = Coverage::Partial(
        "no filter reads an ability's source or whether it is activated, so \
         \"target activated ability from an artifact source\" cannot be \
         stated; an unrestricted counter would be stronger than the card \
         prints"
    ),
    faces = &[face!(
        name = "Ayesha Tanaka",
        mana_cost = mana!("{W}{W}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(2),
        toughness = Some(2),
    ),],
);
