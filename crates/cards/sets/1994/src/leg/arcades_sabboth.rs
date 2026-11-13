//! Arcades Sabboth — {2}{G}{G}{W}{W}{U}{U} — Legendary Creature — Elder Dragon
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, sacrifice Arcades Sabboth unless you pay {G}{W}{U}.
//! Oracle: Each untapped creature you control gets +0/+2 as long as it's not attacking.
//! Oracle: {W}: Arcades Sabboth gets +0/+1 until end of turn.
//! Set: DMR #187 — Dominaria Remastered | Scryfall ID: 175695bb-2630-4269-a407-9138c7008e81 | Oracle ID: 8ead94c5-8447-4f20-87ec-efdccde689fc
// IMPLEMENTED — flying, upkeep tax, +0/+2 to untapped nonattacking creatures you control, and {W} pump.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCADES_SABBOTH,
    oracle_id = "8ead94c5-8447-4f20-87ec-efdccde689fc",
    scryfall_id = "175695bb-2630-4269-a407-9138c7008e81",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arcades Sabboth",
        mana_cost = mana!("{2}{G}{G}{W}{W}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELDER, subtypes::creature::DRAGON],
        power = Some(7),
        toughness = Some(7),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{G}{W}{U}"),
                effect: &Effect::SacrificeSelf
            }]
        ),
        static_ability!(
            Filter::And(&[
                Filter::CREATURE,
                Filter::Untapped,
                Filter::Not(&Filter::Attacking),
                Filter::ControlledByYou
            ]),
            Modifier::ModifyPT(0, 2)
        ),
        activated!(
            cost!("{W}"),
            &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn
            }]
        ),
    ],
);
