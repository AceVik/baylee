//! Wan Shi Tong, Librarian — {X}{U}{U} — Legendary Creature — Bird Spirit
//! Oracle: Flash
//! Oracle: Flying, vigilance
//! Oracle: When Wan Shi Tong enters, put X +1/+1 counters on him. Then draw half X cards, rounded down.
//! Oracle: Whenever an opponent searches their library, put a +1/+1 counter on Wan Shi Tong and draw a card.
//! Set: TLA #78 — Avatar: The Last Airbender | Scryfall ID: e20da6b5-1057-4a28-9e85-07de714e262f | Oracle ID: 2bace3ea-7d28-4a1c-857a-d3c3a1c11b59

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WAN_SHI_TONG_LIBRARIAN,
    oracle_id = "2bace3ea-7d28-4a1c-857a-d3c3a1c11b59",
    scryfall_id = "e20da6b5-1057-4a28-9e85-07de714e262f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLASH
        .union(KeywordSet::FLYING)
        .union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wan Shi Tong, Librarian",
        mana_cost = mana!("{X}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::X,
                },
                Effect::DrawCards {
                    amount: Amount::Half {
                        base: &Amount::X,
                        round_up: false,
                    },
                },
            ],
        ),
        triggered!(
            Trigger::SearchesLibrary(PlayerRel::Opponent),
            &[
                Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::draw(1),
            ],
        ),
    ],
);
