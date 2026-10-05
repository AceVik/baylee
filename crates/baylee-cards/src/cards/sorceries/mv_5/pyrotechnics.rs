//! Pyrotechnics — {4}{R} — Sorcery
//! Oracle: Pyrotechnics deals 4 damage divided as you choose among any number of targets.
//! Set: FRF #111 — Fate Reforged | Scryfall ID: 51893dd5-e70f-44bb-85d4-e31480ba84d6 | Oracle ID: 794b9ddf-1660-441f-9cc2-a0f5d4d0cf22
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PYROTECHNICS,
    oracle_id = "794b9ddf-1660-441f-9cc2-a0f5d4d0cf22",
    scryfall_id = "51893dd5-e70f-44bb-85d4-e31480ba84d6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "the division question for Effect::DealDamageDivided is asked only \
         as a triggered ability goes on the stack (CR 601.2d, 603.3d), so a \
         sorcery's \"damage divided as you choose\" has no spelling; \
         DealDamageEvenly is \"divided evenly\" and not the same card"
    ),
    faces = &[face!(
        name = "Pyrotechnics",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Pyrotechnics deals 4 damage divided as you choose among
    // any number of targets." — `Effect::DealDamageDivided` carries the
    // sentence but `Engine::ask_trigger_division` asks the division only for
    // a triggered ability (and
    // `lints::every_divided_damage_is_a_trigger_that_can_divide` refuses it
    // anywhere else), while `Effect::DealDamageEvenly` is the printed
    // "divided evenly" and would split 2/2 where the caster may choose 4/0.
    abilities = &[],
);
