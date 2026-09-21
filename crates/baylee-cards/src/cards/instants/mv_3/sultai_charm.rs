//! Sultai Charm — {B}{G}{U} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy target monocolored creature.
//! Oracle: • Destroy target artifact or enchantment.
//! Oracle: • Draw two cards, then discard a card.
//! Set: DMC #168 — Dominaria United Commander | Scryfall ID: 72af6c1f-33a0-4e02-95b7-74ecaa6a6d87 | Oracle ID: 46ed38d1-e642-4cea-99ed-a9c17fd982b1
// IMPLEMENTED — one modal spell: destroy a monocolored creature, destroy an
// artifact or enchantment, or draw two cards then discard one.

use baylee_cards_dsl::prelude::*;

static MONOCOLORED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Monocolored]);

card!(
    index = index::SULTAI_CHARM,
    oracle_id = "46ed38d1-e642-4cea-99ed-a9c17fd982b1",
    scryfall_id = "72af6c1f-33a0-4e02-95b7-74ecaa6a6d87",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Sultai Charm",
        mana_cost = mana!("{B}{G}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        modes: &[
            mode!(
                &[Effect::destroy(TargetSpec::Object(&MONOCOLORED_CREATURE))],
                targets = Some(TargetReq::one(TargetSpec::Object(&MONOCOLORED_CREATURE)))
            ),
            mode!(
                &[Effect::destroy(TargetSpec::Object(
                    &Filter::ARTIFACT_OR_ENCHANTMENT
                ))],
                targets = Some(TargetReq::one(TargetSpec::Object(
                    &Filter::ARTIFACT_OR_ENCHANTMENT
                )))
            ),
            mode!(&[
                Effect::draw(2),
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1,
                },
            ]),
        ],
    }],
);
