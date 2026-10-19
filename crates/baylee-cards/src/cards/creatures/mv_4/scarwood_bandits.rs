//! Scarwood Bandits — {2}{G}{G} — Creature — Human Rogue
//! Oracle: Forestwalk (This creature can't be blocked as long as defending player controls a Forest.)
//! Oracle: {2}{G}, {T}: Unless an opponent pays {2}, gain control of target artifact for as long as this creature remains on the battlefield.
//! Set: ME4 #165 — Masters Edition IV | Scryfall ID: 7ca0e682-22c1-4cd9-9283-fb91ef29a752 | Oracle ID: 7fcc8aab-6f7b-4474-b4e4-58cf4a119b22

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCARWOOD_BANDITS,
    oracle_id = "7fcc8aab-6f7b-4474-b4e4-58cf4a119b22",
    scryfall_id = "7ca0e682-22c1-4cd9-9283-fb91ef29a752",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scarwood Bandits",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FORESTWALK,
        abilities = &[activated!(
            cost!("{2}{G}", TapSelf),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::Opponent,
                mana: Amount::Fixed(2),
                effect: &Effect::ModifyTargets {
                    modifier: Modifier::GainControl,
                    duration: Duration::WhileSourceOnBattlefield,
                },
            }],
            target = Some(TargetSpec::Object(&Filter::ARTIFACT)),
        )],
    ),],
);
