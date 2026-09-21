//! Khalni Ambush // Khalni Territory — {2}{G} — Instant // Land
//! Oracle: Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #192 — Zendikar Rising | Scryfall ID: 99535539-aa73-41ed-86ab-21c97b92620d | Oracle ID: 37a55560-6e32-4f54-b9a8-fd157aea6eb5
//! Face: Khalni Ambush — {2}{G} — Instant
//! Face: Khalni Territory —  — Land
// PARTIAL — the Khalni Territory half is built in full: it enters tapped and
// prints its own {T}: Add {G} (a land with no basic land type, so the
// intrinsic shortcut has nothing to answer with), and the MDFC face choice on
// play is the engine's. The Khalni Ambush half is gone.

// NOT SUPPORTED: Khalni Ambush — "Target creature you control fights target
// creature you don't control. (Each deals damage equal to its power to the
// other.)" There is no fight effect in the vocabulary — the sentence is two
// DealDamage halves that each read the *other* creature's power, and
// Amount::TargetPower names the first target only — and a TargetReq carries a
// single spec, so "you control" and "you don't control" cannot be required
// together on one cast either way.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card!(
    index = index::KHALNI_AMBUSH,
    oracle_id = "37a55560-6e32-4f54-b9a8-fd157aea6eb5",
    scryfall_id = "99535539-aa73-41ed-86ab-21c97b92620d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Khalni Ambush",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Khalni Territory",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial("Khalni Ambush's fight clause"),
);
