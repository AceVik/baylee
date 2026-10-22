//! Hall of Gemstone — {1}{G}{G} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses a color. Until end of turn, lands tapped for mana produce mana of the chosen color instead of any other color.
//! Set: MIR #221 — Mirage | Scryfall ID: 18e4551f-9f6c-4421-ad66-a270df6d3463 | Oracle ID: 3da9fecc-064a-44e6-b88e-4b10194e367e
// NOT SUPPORTED: letting the active player choose which color lands produce until end of turn; replaces colored mana produced by lands tapped for mana with {G} until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_GEMSTONE,
    oracle_id = "3da9fecc-064a-44e6-b88e-4b10194e367e",
    scryfall_id = "18e4551f-9f6c-4421-ad66-a270df6d3463",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "at the beginning of each player's upkeep, replaces colored mana produced by lands tapped for mana with {G} until end of turn; letting the active player choose which color lands produce is not in Effect",
    ),
    faces = &[face!(
        name = "Hall of Gemstone",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[
            Effect::continuous(
                &Filter::LAND,
                Modifier::ReplaceTappedMana {
                    from: ManaColor::White,
                    to: ManaColor::Green,
                },
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::LAND,
                Modifier::ReplaceTappedMana {
                    from: ManaColor::Blue,
                    to: ManaColor::Green,
                },
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::LAND,
                Modifier::ReplaceTappedMana {
                    from: ManaColor::Black,
                    to: ManaColor::Green,
                },
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::LAND,
                Modifier::ReplaceTappedMana {
                    from: ManaColor::Red,
                    to: ManaColor::Green,
                },
                Duration::UntilEndOfTurn,
            ),
        ],
    )],
);
