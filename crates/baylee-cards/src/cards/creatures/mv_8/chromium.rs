//! Chromium — {2}{W}{W}{U}{U}{B}{B} — Legendary Creature — Elder Dragon
//! Oracle: Flying
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Oracle: At the beginning of your upkeep, sacrifice Chromium unless you pay {W}{U}{B}.
//! Set: ME3 #147 — Masters Edition III | Scryfall ID: 15ec5a20-4e8f-40b2-9abf-c0bf1cf816c3 | Oracle ID: 7c91ae5d-0320-46a7-98d2-df0918202478
// PARTIAL — flying and the upkeep tax are written; rampage 2 is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHROMIUM,
    oracle_id = "7c91ae5d-0320-46a7-98d2-df0918202478",
    scryfall_id = "15ec5a20-4e8f-40b2-9abf-c0bf1cf816c3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "rampage 2 has no DSL vocabulary: no rampage keyword bit, no \
         \"becomes blocked\" trigger, and no amount counting the blockers \
         beyond the first"
    ),
    faces = &[face!(
        name = "Chromium",
        mana_cost = mana!("{2}{W}{W}{U}{U}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELDER, subtypes::creature::DRAGON],
        power = Some(7),
        toughness = Some(7),
    ),],
    // NOT SUPPORTED: "Rampage 2 (Whenever this creature becomes blocked, it
    // gets +2/+2 until end of turn for each creature blocking it beyond the
    // first.)" — the DSL has no rampage keyword (`KeywordSet` carries only
    // text-independent bits, and rampage carries a number), no trigger for
    // "becomes blocked" (`Trigger::BlocksOrBecomesBlockedBy` also fires when
    // this creature blocks and takes a filter rather than a count), and no
    // `Amount` counts "each creature blocking it beyond the first".
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: mana!("{W}{U}{B}"),
            effect: &Effect::SacrificeSelf
        }]
    )],
);
