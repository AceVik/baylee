//! Disturbed Burial — {1}{B} — Sorcery
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Return target creature card from your graveyard to your hand.
//! Set: TMP #129 — Tempest | Scryfall ID: 06254b6c-eb22-4ec9-9420-74e9ee15e072 | Oracle ID: cdbaa21d-5896-4b8b-9a08-f0839ccfddfc
// PARTIAL — Returns target creature card from your graveyard to your hand; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISTURBED_BURIAL,
    oracle_id = "cdbaa21d-5896-4b8b-9a08-f0839ccfddfc",
    scryfall_id = "06254b6c-eb22-4ec9-9420-74e9ee15e072",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Returns target creature card from your graveyard to your hand; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Disturbed Burial",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )],
);
