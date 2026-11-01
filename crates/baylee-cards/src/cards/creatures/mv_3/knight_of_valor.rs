//! Knight of Valor — {2}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {1}{W}: Each creature without flanking blocking this creature gets -1/-1 until end of turn. Activate only once each turn.
//! Set: VIS #11 — Visions | Scryfall ID: 25aa80ae-bb17-4e52-a269-efe75cf4c041 | Oracle ID: f2edb543-de33-4eed-892c-98038eec96e2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKER_WITHOUT_FLANKING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::BlockingSource,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLANKING)),
]);

card!(
    index = index::KNIGHT_OF_VALOR,
    oracle_id = "f2edb543-de33-4eed-892c-98038eec96e2",
    scryfall_id = "25aa80ae-bb17-4e52-a269-efe75cf4c041",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Knight of Valor",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[activated!(
        cost!("{1}{W}"),
        &[Effect::continuous(
            &BLOCKER_WITHOUT_FLANKING,
            Modifier::ModifyPT(-1, -1),
            Duration::UntilEndOfTurn,
        )],
        limit = ActivationLimit::PerTurn(1),
    ),],
);
