//! Freyalise's Charm — {G}{G} — Enchantment
//! Oracle: Whenever an opponent casts a black spell, you may pay {G}{G}. If you do, you draw a card.
//! Oracle: {G}{G}: Return this enchantment to its owner's hand.
//! Set: ICE #240 — Ice Age | Scryfall ID: 3e147ac1-d221-49c7-966e-5e665ddeab6b | Oracle ID: e0d638d2-2c5b-415c-95e7-74bf8d88794f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

const OPPONENT_BLACK_SPELL: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::FREYALISE_S_CHARM,
    oracle_id = "e0d638d2-2c5b-415c-95e7-74bf8d88794f",
    scryfall_id = "3e147ac1-d221-49c7-966e-5e665ddeab6b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Freyalise's Charm",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::SpellCast(&OPPONENT_BLACK_SPELL),
            &[Effect::PlayerMayPayManaThen {
                player: PlayerRel::You,
                cost: mana!("{G}{G}"),
                effects: &[Effect::draw(1)],
            }],
        ),
        activated!(
            cost!("{G}{G}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
