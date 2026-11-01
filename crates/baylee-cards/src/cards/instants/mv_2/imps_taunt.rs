//! Imps' Taunt — {1}{B} — Instant
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target creature attacks this turn if able.
//! Set: TMP #138 — Tempest | Scryfall ID: 79d6ed64-9f5c-4233-85a9-028b8e5949c3 | Oracle ID: 7ebec169-d941-460e-9e3c-ba54b5066b05
// PARTIAL — Forces target creature to attack this turn if able; the buyback {3} option is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMPS_TAUNT,
    oracle_id = "7ebec169-d941-460e-9e3c-ba54b5066b05",
    scryfall_id = "79d6ed64-9f5c-4233-85a9-028b8e5949c3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Forces target creature to attack this turn if able; the buyback {3} option is not yet modeled."
    ),
    faces = &[face!(
        name = "Imps' Taunt",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::AttacksEachCombat,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
