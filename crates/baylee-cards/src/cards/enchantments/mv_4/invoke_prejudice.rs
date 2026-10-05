//! Invoke Prejudice — {U}{U}{U}{U} — Enchantment
//! Oracle: Whenever an opponent casts a creature spell that doesn't share a color with a creature you control, counter that spell unless that player pays {X}, where X is its mana value.
//! Set: LEG #62 — Legends | Scryfall ID: 903d9fde-d7da-4a0e-a337-b63023c6d74b | Oracle ID: 854ad486-0c59-4c57-9a76-ab1dff0ff37c
// PARTIAL — the trigger is off the card: the colour comparison that gates it
// cannot be written, so any spelling would counter spells the printed card
// lets through (see the NOT SUPPORTED line).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INVOKE_PREJUDICE,
    oracle_id = "854ad486-0c59-4c57-9a76-ab1dff0ff37c",
    scryfall_id = "903d9fde-d7da-4a0e-a337-b63023c6d74b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no Filter predicate compares the candidate's colors with another \
         object's, so \"doesn't share a color with a creature you control\" \
         cannot gate the trigger and the ability is off"
    ),
    faces = &[face!(
        name = "Invoke Prejudice",
        mana_cost = mana!("{U}{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "Whenever an opponent casts a creature spell that doesn't
    // share a color with a creature you control, counter that spell unless
    // that player pays {X}, where X is its mana value." — `Filter::HasColor`
    // takes a fixed `ColorSet` and there is no predicate that compares a
    // spell's colors with the colors of a creature its controller controls
    // (`SharesSubtypeWithCommander` is the only shares-something filter), so
    // the spell filter cannot be written. The rest of the trigger would be
    // sayable (`PlayerMayPayOr` with `Amount::TargetCmc`), but without the
    // gate it would counter every opponent's creature spell, a stronger card.
    abilities = &[],
);
