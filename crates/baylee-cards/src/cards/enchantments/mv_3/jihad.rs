//! Jihad — {W}{W}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a color and an opponent.
//! Oracle: White creatures get +2/+1 as long as the chosen player controls a nontoken permanent of the chosen color.
//! Oracle: When the chosen player controls no nontoken permanents of the chosen color, sacrifice this enchantment.
//! Set: ARN #5 — Arabian Nights | Scryfall ID: b6c7705a-2987-4ef1-92b1-2c55d989ec6f | Oracle ID: b18b9869-8490-4875-a5bb-484c3299f2c5
// PARTIAL — no ability is written.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JIHAD,
    oracle_id = "b18b9869-8490-4875-a5bb-484c3299f2c5",
    scryfall_id = "b6c7705a-2987-4ef1-92b1-2c55d989ec6f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Jihad",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "no Filter or Condition reads back the chosen colour or the chosen \
         player, so neither the anthem nor its state trigger can be stated"
    ),
    // NOT SUPPORTED: "As this enchantment enters, choose a color and an
    // opponent. White creatures get +2/+1 as long as the chosen player
    // controls a nontoken permanent of the chosen color. When the chosen
    // player controls no nontoken permanents of the chosen color, sacrifice
    // this enchantment." — `EnterModifier::ChooseColor` and
    // `EnterModifier::ChooseOpponent` both exist, but the entry scan applies
    // one question (`lints::no_face_asks_two_questions_as_it_enters`), and
    // the answers are read back only where `chosen_color` names mana and
    // `PlayerRel::Chosen` names a resolution's player: no `Filter` matches
    // "a permanent of the chosen color" and no `Condition` says "the chosen
    // player controls …". The anthem and the state trigger are therefore
    // off the card, and the entry question is not asked for nothing.
);
