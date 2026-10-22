//! Anoint — {W} — Instant
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Prevent the next 3 damage that would be dealt to target creature this turn.
//! Set: TPR #3 — Tempest Remastered | Scryfall ID: 193af7bd-547b-489b-bf21-6a2873311afc | Oracle ID: fcb03c54-5b9a-428d-bbf6-ef8977770c8d
// PARTIAL — Prevents the next 3 damage that would be dealt to target creature this turn; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANOINT,
    oracle_id = "fcb03c54-5b9a-428d-bbf6-ef8977770c8d",
    scryfall_id = "193af7bd-547b-489b-bf21-6a2873311afc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Prevents the next 3 damage that would be dealt to target creature this turn; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Anoint",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::Fixed(3),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
