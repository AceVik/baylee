//! `cards/creatures/mv_4/root_greevil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn root_greevil() -> CardIndex {
    card_index("e43b25be-fe67-4eaa-80df-85718f38f904")
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

fn levitation() -> CardIndex {
    card_index("94b703c4-5584-4913-8365-7e9f2f535c2d")
}

fn shu_general() -> CardIndex {
    card_index("0fef6729-b044-4daf-834a-ac8c9711ad8a")
}

/// Root Greevil (e43b25be-fe67-4eaa-80df-85718f38f904) — `{3}{G}` 2/3 Creature — Beast:
/// "`{2}{G}`, `{T}`, Sacrifice this creature: Destroy all enchantments of the color of your choice."
#[test]
fn root_greevil_sacrifices_to_destroy_only_enchantments_of_chosen_color() {
    let lands = vec![forest(); 7];
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lands)
        .battlefield(1, &[glorious_anthem(), levitation(), shu_general()])
        .hand(0, &[root_greevil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, root_greevil());
    pass_until(&mut engine, stack_is_empty);

    let greevil = on_battlefield(&engine, P0, root_greevil()).expect("Root Greevil entered");
    assert_eq!(pt(&engine, greevil), (2, 3));

    // Wait out summoning sickness so `{T}` cost is legal, then sacrifice choosing White.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, root_greevil(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, root_greevil()).is_some());
    assert!(
        in_graveyard(&engine, P1, glorious_anthem()).is_some(),
        "white enchantment is destroyed"
    );
    assert!(
        on_battlefield(&engine, P1, levitation()).is_some(),
        "blue enchantment is untouched"
    );
    assert!(
        on_battlefield(&engine, P1, shu_general()).is_some(),
        "white non-enchantment permanent is untouched"
    );
}
