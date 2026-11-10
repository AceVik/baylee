//! Murk Dwellers — {3}{B} — Creature — Zombie
//! Oracle: Whenever this creature attacks and isn't blocked, it gets +2/+0 until end of combat.
//! Set: 5ED #180 — Fifth Edition | Scryfall ID: 740564ec-c473-45bc-ba94-288786bf28b9 | Oracle ID: 00358a89-2058-448b-a2fd-389f639ba2da

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MURK_DWELLERS,
    oracle_id = "00358a89-2058-448b-a2fd-389f639ba2da",
    scryfall_id = "740564ec-c473-45bc-ba94-288786bf28b9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Murk Dwellers",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, 0),
            duration: Duration::UntilEndOfCombat,
        }],
    )],
);
