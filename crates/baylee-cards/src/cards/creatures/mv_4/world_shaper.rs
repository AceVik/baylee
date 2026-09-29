//! World Shaper — {3}{G} — Creature — Merfolk Shaman
//! Oracle: Whenever this creature attacks, you may mill three cards.
//! Oracle: When this creature dies, return all land cards from your graveyard to the battlefield tapped.
//! Set: OTC #214 — Outlaws of Thunder Junction Commander | Scryfall ID: cc765da4-4bca-4250-80e4-05575d6fa98c | Oracle ID: 3c075bb6-1831-4521-bd8d-4ed2825ae796
// IMPLEMENTED — the attack trigger mills three if its controller says so;
// the dies trigger returns every land card from its controller's graveyard
// to the battlefield tapped (Lumra's sweep).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORLD_SHAPER,
    oracle_id = "3c075bb6-1831-4521-bd8d-4ed2825ae796",
    scryfall_id = "cc765da4-4bca-4250-80e4-05575d6fa98c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "World Shaper",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::MayDo {
                effects: &[Effect::Mill {
                    amount: Amount::Fixed(3),
                    target: PlayerRel::You,
                }],
            }]
        ),
        // "When this creature dies, return all land cards from your
        // graveyard to the battlefield tapped."
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnAllFromGraveyard {
                filter: &Filter::LAND,
                tapped: true,
            }]
        ),
    ],
);
