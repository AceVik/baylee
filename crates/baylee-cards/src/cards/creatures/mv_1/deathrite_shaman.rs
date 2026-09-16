//! Deathrite Shaman — {B/G} — Creature — Elf Shaman
//! Oracle: {T}: Exile target land card from a graveyard. Add one mana of any color. (Activate only as an instant.)
//! Oracle: {B}, {T}: Exile target instant or sorcery card from a graveyard. Each opponent loses 2 life.
//! Oracle: {G}, {T}: Exile target creature card from a graveyard. You gain 2 life.
//! Set: RVR #175 — Ravnica Remastered | Scryfall ID: cfdb1c47-14be-491f-88b3-bed03489dbc5 | Oracle ID: 22f1a4a4-c423-4d1c-8775-0ed604a9fa51
// IMPLEMENTED — three graveyard-eating abilities, each exiling a card of one
// kind out of any graveyard: a land for one mana of any colour, an instant or
// sorcery for 2 life off every opponent, a creature for 2 life gained.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEATHRITE_SHAMAN,
    oracle_id = "22f1a4a4-c423-4d1c-8775-0ed604a9fa51",
    scryfall_id = "cfdb1c47-14be-491f-88b3-bed03489dbc5",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Deathrite Shaman",
        mana_cost = mana!("{B/G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // "(Activate only as an instant.)" is the printed reminder that this
        // is *not* a mana ability: it targets, and CR 605.1a puts a targeted
        // ability on the stack however much mana it makes. Both halves of
        // that are the macro's defaults — `mana_ability = false` (CR 605.1)
        // and instant-speed timing (CR 602.2) — so neither is restated.
        activated!(
            Cost::TAP,
            &[
                Effect::Exile {
                    target: TargetSpec::CardInGraveyard(&Filter::LAND, PlayerRel::EachPlayer),
                },
                Effect::mana_of_any_color(),
            ],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::LAND,
                PlayerRel::EachPlayer
            ))
        ),
        activated!(
            cost!("{B}", TapSelf),
            &[
                Effect::Exile {
                    target: TargetSpec::CardInGraveyard(
                        &Filter::INSTANT_OR_SORCERY,
                        PlayerRel::EachPlayer,
                    ),
                },
                Effect::LoseLife {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::EachOpponent,
                },
            ],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::INSTANT_OR_SORCERY,
                PlayerRel::EachPlayer
            ))
        ),
        activated!(
            cost!("{G}", TapSelf),
            &[
                Effect::Exile {
                    target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer),
                },
                Effect::GainLife {
                    amount: Amount::Fixed(2),
                },
            ],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::EachPlayer
            ))
        ),
    ],
);

// Engine-level test belongs in `card_tests` (deathrite_shaman_eats_a_graveyard):
// no card in the pool exiles a card *out of a graveyard* yet, so `Effect::Exile`
// aimed at a `CardInGraveyard` target is played here for the first time — and
// "from a graveyard" is `PlayerRel::EachPlayer`, so an opponent's graveyard has
// to be on the offer as well as your own.
