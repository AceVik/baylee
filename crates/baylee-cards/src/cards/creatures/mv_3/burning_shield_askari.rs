//! Burning Shield Askari — {2}{R} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {R}{R}: This creature gains first strike until end of turn.
//! Set: MIR #162 — Mirage | Scryfall ID: 486547cd-d2e7-4c46-9f7b-81c4267d65cc | Oracle ID: 042fd95a-5e85-4ce5-bb68-066a295c54fb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BURNING_SHIELD_ASKARI,
    oracle_id = "042fd95a-5e85-4ce5-bb68-066a295c54fb",
    scryfall_id = "486547cd-d2e7-4c46-9f7b-81c4267d65cc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Burning Shield Askari",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[activated!(
        cost!("{R}{R}"),
        &[Effect::ModifySource {
            modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
