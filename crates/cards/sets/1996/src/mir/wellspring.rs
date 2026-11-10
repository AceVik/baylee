//! Wellspring — {1}{G}{W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: When this Aura enters, gain control of enchanted land until end of turn.
//! Oracle: At the beginning of your upkeep, untap enchanted land. You gain control of that land until end of turn.
//! Set: MIR #288 — Mirage | Scryfall ID: c69ba095-dd78-4999-87a9-63f7165846e4 | Oracle ID: 8650bd97-ecf6-420f-9b72-a749970afcd4
// IMPLEMENTED — enchant land, ETB trigger gaining control of enchanted land until end of turn, and upkeep trigger untapping enchanted land and gaining control of it until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WELLSPRING,
    oracle_id = "8650bd97-ecf6-420f-9b72-a749970afcd4",
    scryfall_id = "c69ba095-dd78-4999-87a9-63f7165846e4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wellspring",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
        ),
        triggered!(
            Trigger::ETB,
            &[Effect::continuous(
                &Filter::AttachedToBySource,
                Modifier::GainControl,
                Duration::UntilEndOfTurn,
            )],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::UntapAll {
                    filter: &Filter::AttachedToBySource,
                },
                Effect::continuous(
                    &Filter::AttachedToBySource,
                    Modifier::GainControl,
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
