//! Touch of Vitae — {2}{G} — Instant
//! Oracle: Until end of turn, target creature gains haste and "{0}: Untap this creature. Activate only once."
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ICE #271 — Ice Age | Scryfall ID: 48d2cd18-a24d-40e0-a654-777d9e623ae2 | Oracle ID: 84f1e596-f56f-48c9-9bff-2c0bb2cea2d3

use baylee_cards_dsl::prelude::*;

static UNTAP_ONCE: AbilityDef = activated!(
    cost!("{0}"),
    &[Effect::UntapSelf],
    limit = ActivationLimit::PerTurn(1)
);

card!(
    index = index::TOUCH_OF_VITAE,
    oracle_id = "84f1e596-f56f-48c9-9bff-2c0bb2cea2d3",
    scryfall_id = "48d2cd18-a24d-40e0-a654-777d9e623ae2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Touch of Vitae",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::GrantActivatedDefinition(&UNTAP_ONCE),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
