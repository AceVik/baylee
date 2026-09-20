//! Hostile Hostel // Creeping Inn — (no cost) — Land // Artifact Creature — Horror Construct
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice a creature: Put a soul counter on this land. Then if there are three or more soul counters on it, remove those counters, transform it, then untap it. Activate only as a sorcery.
//! Oracle: Whenever this creature attacks, you may exile a creature card from your graveyard. If you do, each opponent loses X life and you gain X life, where X is the number of creature cards exiled with this creature.
//! Oracle: {4}: This creature phases out.
//! Set: MID #264 — Innistrad: Midnight Hunt | Scryfall ID: ac83c27f-55d6-4e5a-93a4-febb0c183289 | Oracle ID: 1b340f71-502f-48e9-85ed-9af62f356115
//! Face: Hostile Hostel —  — Land
//! Face: Creeping Inn —  — Artifact Creature — Horror Construct
// PARTIAL — the front's {T}: Add {C} and the back's {4}: this creature phases
// out are built; the soul-counter transform and the linked-exile attack
// trigger have no spelling in the vocabulary.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "{1}, {T}, Sacrifice a creature: Put a soul counter on this land. Then if there
// are three or more soul counters on it, remove those counters, transform it, then untap it."
// — the whole activated ability is dropped: `counters` assigns no id for a soul counter, no
// `Effect` removes counters (`RemoveCounterSelf` is a cost), and no effect-level branch asks
// "three or more counters on this" (`IfNoCountersOnSelf` is the nought case only).
// NOT SUPPORTED: "Whenever this creature attacks, you may exile a creature card from your
// graveyard. If you do, each opponent loses X life and you gain X life, where X is the number of
// creature cards exiled with this creature."
// — the trigger is dropped: X counts the cards exiled *with this creature*, and no `Amount` reads
// that link (`CountOf` reaches a battlefield, a library, a graveyard or a hand, never a link).

static BACK_ABILITIES: &[AbilityDef] = &[activated!(
    cost!("{4}"),
    &[Effect::PhaseOut { target: None }]
)];

card!(
    index = index::HOSTILE_HOSTEL,
    oracle_id = "1b340f71-502f-48e9-85ed-9af62f356115",
    scryfall_id = "ac83c27f-55d6-4e5a-93a4-febb0c183289",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "front's soul-counter transform ability and back's linked-exile attack trigger \
         are not expressible",
    ),
    faces = &[
        face!(name = "Hostile Hostel", types = TypeSet::LAND,),
        face!(
            name = "Creeping Inn",
            types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            subtypes = &[subtypes::creature::HORROR, subtypes::creature::CONSTRUCT],
            power = Some(3),
            toughness = Some(7),
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
