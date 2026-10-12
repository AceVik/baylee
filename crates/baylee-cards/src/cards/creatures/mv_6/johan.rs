//! Johan — {3}{R}{G}{W} — Legendary Creature — Human Wizard
//! Oracle: At the beginning of combat on your turn, you may have Johan gain "Johan can't attack" until end of combat. If you do, attacking doesn't cause creatures you control to tap this combat if Johan is untapped.
//! Set: CHR #77 — Chronicles | Scryfall ID: 2f2f3b3e-63f3-4cab-aa95-030990157ed5 | Oracle ID: ad33ada7-4f89-4cab-88db-1f4d024baa36

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JOHAN,
    oracle_id = "ad33ada7-4f89-4cab-88db-1f4d024baa36",
    scryfall_id = "2f2f3b3e-63f3-4cab-aa95-030990157ed5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Johan",
        mana_cost = mana!("{3}{R}{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::You
        },
        &[Effect::MayDo {
            effects: &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::CANT_ATTACK),
                    Duration::UntilEndOfCombat
                ),
                Effect::AttackWithoutTappingWhileSourceUntappedThisCombat,
            ]
        }]
    )],
);
