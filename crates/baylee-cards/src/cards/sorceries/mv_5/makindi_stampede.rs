//! Makindi Stampede // Makindi Mesas — {3}{W}{W} — Sorcery // Land
//! Oracle: Creatures you control get +2/+2 until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #26 — Zendikar Rising | Scryfall ID: ada9a974-8f1f-4148-bd61-200fc14714b2 | Oracle ID: 342e08f9-d4d0-4408-8621-66e087058616
//! Face: Makindi Stampede — {3}{W}{W} — Sorcery
//! Face: Makindi Mesas —  — Land
// IMPLEMENTED — front face pumps your team +2/+2 until end of turn; back
// face enters tapped (EnterModifier::Tapped) and taps for {W}.

use baylee_cards_dsl::prelude::*;

static MESAS_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::MAKINDI_STAMPEDE,
    oracle_id = "342e08f9-d4d0-4408-8621-66e087058616",
    scryfall_id = "ada9a974-8f1f-4148-bd61-200fc14714b2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Makindi Stampede",
            mana_cost = mana!("{3}{W}{W}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Makindi Mesas",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = MESAS_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(2),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }])],
);
