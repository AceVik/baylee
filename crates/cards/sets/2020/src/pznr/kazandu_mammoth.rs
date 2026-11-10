//! Kazandu Mammoth // Kazandu Valley — {1}{G}{G} — Creature — Elephant // Land
//! Oracle: Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #189 — Zendikar Rising | Scryfall ID: 2f632537-63bf-4490-86e6-e6067b9c1a3b | Oracle ID: 2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7
//! Face: Kazandu Mammoth — {1}{G}{G} — Creature — Elephant
//! Face: Kazandu Valley —  — Land
// IMPLEMENTED — landfall pumps the Elephant +2/+2 until end of turn
// ("Landfall" is an ability word, CR 207.2c, with no rules meaning of its
// own); the MDFC land back enters tapped and taps for {G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static VALLEY_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card!(
    index = index::KAZANDU_MAMMOTH,
    oracle_id = "2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7",
    scryfall_id = "2f632537-63bf-4490-86e6-e6067b9c1a3b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Kazandu Mammoth",
            mana_cost = mana!("{1}{G}{G}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::ELEPHANT],
            power = Some(3),
            toughness = Some(3),
        ),
        face!(
            name = "Kazandu Valley",
            types = TypeSet::LAND,
            abilities = VALLEY_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    abilities = &[
        // The trigger names no target, so `Filter::This` is the source — the
        // Mammoth itself rather than something it pointed at. That is the
        // case `Resolution::targeted` exists for, and this is the sentence
        // its doc comment quotes.
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(2, 2),
                Duration::UntilEndOfTurn
            )]
        ),
    ],
);
