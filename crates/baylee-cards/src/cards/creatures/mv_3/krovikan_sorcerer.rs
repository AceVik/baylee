//! Krovikan Sorcerer — {2}{U} — Creature — Human Wizard Sorcerer
//! Oracle: {T}, Discard a nonblack card: Draw a card.
//! Oracle: {T}, Discard a black card: Draw two cards, then discard one of them.
//! Set: VMA #77 — Vintage Masters | Scryfall ID: 62909d11-9d2f-42a7-9b73-6b494bbe3f9e | Oracle ID: 00e6808c-1f9e-4b6a-807c-83f433b81a41
// NOT SUPPORTED: the second ability draws two cards and discards any one card from your hand rather than restricting the discard to one of the two cards drawn this way.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONBLACK_CARD: Filter =
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black])));
static BLACK_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));

card!(
    index = index::KROVIKAN_SORCERER,
    oracle_id = "00e6808c-1f9e-4b6a-807c-83f433b81a41",
    scryfall_id = "62909d11-9d2f-42a7-9b73-6b494bbe3f9e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the second ability draws two cards and discards any one card from your hand rather than restricting the discard to one of the two cards drawn this way",
    ),
    faces = &[face!(
        name = "Krovikan Sorcerer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::SORCERER,
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(cost!(TapSelf, Discard(&NONBLACK_CARD)), &[Effect::draw(1)],),
        activated!(
            cost!(TapSelf, Discard(&BLACK_CARD)),
            &[
                Effect::draw(2),
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1,
                },
            ],
        ),
    ],
);
