//! Torrent of Lava — {X}{R}{R} — Sorcery
//! Oracle: Torrent of Lava deals X damage to each creature without flying.
//! Oracle: As long as Torrent of Lava is on the stack, each creature has "{T}: Prevent the next 1 damage that would be dealt to this creature by Torrent of Lava this turn."
//! Set: MIR #199 — Mirage | Scryfall ID: 19528a24-4968-4742-a2d1-06f94e60f290 | Oracle ID: f467ec85-ffe2-4963-82e0-509a7873fd63
// PARTIAL — deals X damage to each creature without flying; no stack-zone static ability granting each creature "{T}: Prevent the next 1 damage from Torrent of Lava" exists yet.

use baylee_cards_dsl::prelude::*;

static CREATURE_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::TORRENT_OF_LAVA,
    oracle_id = "f467ec85-ffe2-4963-82e0-509a7873fd63",
    scryfall_id = "19528a24-4968-4742-a2d1-06f94e60f290",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "deals X damage to each creature without flying; no stack-zone static ability granting each creature '{T}: Prevent the next 1 damage from Torrent of Lava' exists yet",
    ),
    faces = &[face!(
        name = "Torrent of Lava",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DealDamageEach {
        amount: Amount::X,
        filter: &CREATURE_WITHOUT_FLYING,
    }])],
);
