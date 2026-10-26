//! `cards/artifacts/mv_3/urza_s_incubator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn urza_s_incubator() -> CardIndex {
    card_index("b380c04b-0bce-4328-8f0a-1425e48bee72")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn hurloon_minotaur() -> CardIndex {
    card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
}

#[test]
fn urzas_incubator_reduces_chosen_creature_subtype_cost_by_two_for_all_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[urza_s_incubator(), hill_giant(), hurloon_minotaur()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap three Mountains to cast Urza's Incubator ({3}), choosing Giant.
    let p0_lands = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| p0_lands[..3].contains(&id));
    cast_with_floating(&mut engine, p0, urza_s_incubator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::GIANT),
        )
        .expect("Giant is a creature subtype");
    pass_until(&mut engine, stack_is_empty);

    // Negative boundary: Hurloon Minotaur ({1}{R}{R} Minotaur) is not a Giant,
    // so with only 2 Mountains left untapped (2 red mana total), Hurloon
    // Minotaur cannot be cast, whereas Hill Giant ({3}{R} Giant, discounted by
    // {2} to {1}{R}) CAN be cast off those same two Mountains.
    tap_mana_where(&mut engine, p0, |id| p0_lands[3..5].contains(&id));
    let minotaur_in_hand = in_hand(&engine, p0, hurloon_minotaur()).unwrap();
    let giant_in_hand = in_hand(&engine, p0, hill_giant()).unwrap();
    let castable = &priority_offer(&engine).castable;
    assert!(
        !castable.contains(&minotaur_in_hand),
        "non-Giant creature spell does not get the {{2}} discount"
    );
    assert!(
        castable.contains(&giant_in_hand),
        "Giant creature spell costs {{2}} less ({{3}}{{R}} -> {{1}}{{R}})"
    );
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: giant_in_hand,
            },
        )
        .expect("Hill Giant casts for {1}{R}");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, hill_giant()).is_some());

    // Symmetric check: on p1's turn, p1 also gets the {2} discount on Giant
    // creature spells and casts Hill Giant ({3}{R}) with only two Mountains.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, hill_giant());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, hill_giant()).is_some(),
        "Urza's Incubator discounts creature spells of the chosen type for opponents too"
    );
}
