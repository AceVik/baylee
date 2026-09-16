//! Smuggler's Copter — {2} — Artifact — Vehicle
//! Oracle: Flying
//! Oracle: Whenever this Vehicle attacks or blocks, you may draw a card. If you do, discard a card.
//! Oracle: Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)
//! Set: NEC #160 — Neon Dynasty Commander | Scryfall ID: 2680ed41-da35-475a-9d80-ae2f4686feed | Oracle ID: 49136bdc-bc50-49a2-999a-1ef9c16ea130
// PARTIAL — flying, plus the attacking half of the attack-or-block trigger
// (you may draw a card, and if you do, discard a card). Crew, and the
// "or blocks" half, are not expressible; both are flagged below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SMUGGLER_S_COPTER,
    oracle_id = "49136bdc-bc50-49a2-999a-1ef9c16ea130",
    scryfall_id = "2680ed41-da35-475a-9d80-ae2f4686feed",
    faces = &[face!(
        name = "Smuggler's Copter",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::VEHICLE],
        power = Some(3),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial("Crew 1; the \"or blocks\" half of the attack-or-block trigger",),
    abilities = &[
        // NOT SUPPORTED: "or blocks" — Trigger has no variant for a permanent
        // blocking. Trigger::Attacks is the nearest that exists and is the
        // half written here; the ability therefore fires only while something
        // animates this Vehicle into a creature.
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::MayDo {
                effects: &[
                    Effect::draw(1),
                    Effect::DiscardForPlayers {
                        who: PlayerRel::You,
                        count: 1,
                    },
                ],
            }],
        ),
    ],
);

// NOT SUPPORTED: Crew 1 — "Tap any number of creatures you control with
// total power 1 or more" is no CostPart (CostPart::TapOther, the nearest,
// taps exactly one permanent named by a filter), so the cost cannot be
// written or paid; and nothing turns that payment into "this Vehicle becomes
// an artifact creature until end of turn", whose nearest spelling is
// Effect::continuous(&Filter::This, Modifier::AddType(TypeSet::CREATURE),
// Duration::UntilEndOfTurn).
