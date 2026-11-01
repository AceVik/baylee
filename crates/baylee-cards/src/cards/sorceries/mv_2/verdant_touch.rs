//! Verdant Touch — {1}{G} — Sorcery
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target land becomes a 2/2 creature that's still a land. (This effect lasts indefinitely.)
//! Set: TPR #203 — Tempest Remastered | Scryfall ID: c19849f9-cb47-42c9-aaf8-e2e90e157f6e | Oracle ID: 68845200-5346-473d-becd-27315879443d
// PARTIAL: buyback {3} is not supported; makes target land a 2/2 creature that's still a land indefinitely.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VERDANT_TOUCH,
    oracle_id = "68845200-5346-473d-becd-27315879443d",
    scryfall_id = "c19849f9-cb47-42c9-aaf8-e2e90e157f6e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "buyback {3} is not supported; makes target land a 2/2 creature that's still a land indefinitely",
    ),
    faces = &[face!(
        name = "Verdant Touch",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::ModifyTargets {
                modifier: Modifier::AddType(TypeSet::CREATURE),
                duration: Duration::Indefinitely
            },
            Effect::ModifyTargets {
                modifier: Modifier::SetPT(2, 2),
                duration: Duration::Indefinitely
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    ),],
);
