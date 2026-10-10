//! Razorgrass Ambush // Razorgrass Field — {1}{W} — Instant // Land
//! Oracle: Razorgrass Ambush deals 3 damage to target attacking or blocking creature.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: MH3 #238 — Modern Horizons 3 | Scryfall ID: 57065dca-f90e-4184-bbc4-95d726a4160b | Oracle ID: 5da954fa-9001-4557-825c-1462035d21ed
//! Face: Razorgrass Ambush — {1}{W} — Instant
//! Face: Razorgrass Field —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static RAZORGRASS_FIELD: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::RAZORGRASS_AMBUSH,
    oracle_id = "5da954fa-9001-4557-825c-1462035d21ed",
    scryfall_id = "57065dca-f90e-4184-bbc4-95d726a4160b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Razorgrass Ambush",
            mana_cost = mana!("{1}{W}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Razorgrass Field",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = RAZORGRASS_FIELD,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::Or(&[Filter::Attacking, Filter::Blocking])
            ])),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Or(&[Filter::Attacking, Filter::Blocking])
        ]),))),
    )],
);
