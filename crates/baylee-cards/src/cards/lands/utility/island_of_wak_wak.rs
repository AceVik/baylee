//! Island of Wak-Wak — (no cost) — Land
//! Oracle: {T}: Target creature with flying has base power 0 until end of turn.
//! Set: ME1 #176 — Masters Edition | Scryfall ID: 17f4a1aa-395f-423c-b70c-00cf178bb84d | Oracle ID: d427e61d-5b30-4d2a-bad2-2e7f016036ca
// IMPLEMENTED — the target creature with flying gets base power 0 until end
// of turn (Modifier::SetPower, layer 7b), its toughness untouched.

use baylee_cards_dsl::prelude::*;

/// "Creature with flying."
static FLYING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::ISLAND_OF_WAK_WAK,
    oracle_id = "d427e61d-5b30-4d2a-bad2-2e7f016036ca",
    scryfall_id = "17f4a1aa-395f-423c-b70c-00cf178bb84d",
    faces = &[face!(name = "Island of Wak-Wak", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::SetPower(0),
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&FLYING_CREATURE)),
    )],
);
