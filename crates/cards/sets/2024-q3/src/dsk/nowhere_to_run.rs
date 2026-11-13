//! Nowhere to Run — {1}{B} — Enchantment
//! Oracle: Flash
//! Oracle: When this enchantment enters, target creature an opponent controls gets -3/-3 until end of turn.
//! Oracle: Creatures your opponents control can be the targets of spells and abilities as though they didn't have hexproof. Ward abilities of those creatures don't trigger.
//! Set: DSK #111 — Duskmourn: House of Horror | Scryfall ID: fee60e9d-9ee7-444a-88f3-c1929e1888fb | Oracle ID: da82ca96-a613-4d00-9e6b-1fece4fc23d0
// IMPLEMENTED — Flash, an opposing-creature ETB debuff, and the targeting
// permission and Ward suppression while this enchantment is on the battlefield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOWHERE_TO_RUN,
    oracle_id = "da82ca96-a613-4d00-9e6b-1fece4fc23d0",
    scryfall_id = "fee60e9d-9ee7-444a-88f3-c1929e1888fb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nowhere to Run",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::PumpTarget {
                power: Amount::NegXFixed(3),
                toughness: Amount::NegXFixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &Filter::OPPONENT_CREATURE
            )))
        ),
        // One permission for this enchantment's controller, not a removal of
        // anybody's hexproof. The Ward rule affects the opposing creatures.
        static_ability!(Filter::Any, Modifier::IgnoreOpponentsHexproof),
        static_ability!(Filter::OPPONENT_CREATURE, Modifier::WardDoesntTrigger),
    ],
);
