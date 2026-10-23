//! Slaughter — {2}{B}{B} — Instant
//! Oracle: Buyback—Pay 4 life. (You may pay 4 life in addition to any other costs as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Destroy target nonblack creature. It can't be regenerated.
//! Set: EXO #74 — Exodus | Scryfall ID: 8ff06c7d-5e78-4bcf-864b-34487f6555b2 | Oracle ID: 37de575d-15d4-42a2-a3d2-3855fb6ab110
// PARTIAL — Buyback (Pay 4 life) is not supported; destroys target nonblack creature with no regeneration

use baylee_cards_dsl::prelude::*;

static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

// NOT SUPPORTED: Buyback (Pay 4 life) is not supported; destroys target nonblack creature with no regeneration
card!(
    index = index::SLAUGHTER,
    oracle_id = "37de575d-15d4-42a2-a3d2-3855fb6ab110",
    scryfall_id = "8ff06c7d-5e78-4bcf-864b-34487f6555b2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Buyback (Pay 4 life) is not supported; destroys target nonblack creature with no regeneration",
    ),
    faces = &[face!(
        name = "Slaughter",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&NONBLACK_CREATURE),
            no_regen: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONBLACK_CREATURE))),
    )],
);
