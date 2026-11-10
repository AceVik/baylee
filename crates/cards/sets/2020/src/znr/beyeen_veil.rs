//! Beyeen Veil // Beyeen Coast — {1}{U} — Instant // Land
//! Oracle: Creatures your opponents control get -2/-0 until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #46 — Zendikar Rising | Scryfall ID: 5f411f08-45dd-4d73-8894-daf51c175150 | Oracle ID: b03de49d-246f-44e2-9487-9e4e43ec7be4
//! Face: Beyeen Veil — {1}{U} — Instant
//! Face: Beyeen Coast —  — Land
// IMPLEMENTED — Beyeen Veil is a one-sided shrinking spell (-2/-0 to every
// creature your opponents control, until end of turn); Beyeen Coast enters
// tapped and taps for {U}.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::BEYEEN_VEIL,
    oracle_id = "b03de49d-246f-44e2-9487-9e4e43ec7be4",
    scryfall_id = "5f411f08-45dd-4d73-8894-daf51c175150",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Beyeen Veil",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Beyeen Coast",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::OPPONENT_CREATURE,
        controlled_by: None,
        power: Amount::NegXFixed(2),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }])],
);
