//! `cards/artifacts/mv_3/long_forgotten_gohei.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn long_forgotten_gohei() -> CardIndex {
    card_index("897910d5-13a4-4e1d-8f83-235b773b8322")
}

fn kodama_s_reach() -> CardIndex {
    card_index("1593ea18-2f2f-4ab4-83fb-6ccc0bec8a90")
}

fn kami_of_ancient_law() -> CardIndex {
    card_index("e937e0f5-ca35-4dce-9f05-8f2075731628")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Long-Forgotten Gohei (897910d5-13a4-4e1d-8f83-235b773b8322) — {3} Artifact:
/// "Arcane spells you cast cost {1} less to cast."
/// "Spirit creatures you control get +1/+1."
///
/// Pumps your Spirits (+1/+1) while ignoring non-Spirits and opponent's
/// Spirits, and reduces your Arcane spells by {1} (Kodama's Reach {2}{G} ->
/// {1}{G}) while ignoring non-Arcane spells and the opponent's Arcane spells.
#[test]
fn long_forgotten_gohei_pumps_own_spirits_and_discounts_own_arcane_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                long_forgotten_gohei(),
                kami_of_ancient_law(),
                grizzly_bears(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[kodama_s_reach(), long_forgotten_gohei()])
        .battlefield(1, &[kami_of_ancient_law(), forest(), forest()])
        .hand(1, &[kodama_s_reach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_kami = on_battlefield(&engine, p0, kami_of_ancient_law()).unwrap();
    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_kami = on_battlefield(&engine, p1, kami_of_ancient_law()).unwrap();

    // 1. Spirit anthem: own 2/2 Spirit becomes 3/3; non-Spirit Bears & opponent's Spirit stay 2/2.
    assert_eq!(pt(&engine, my_kami), (3, 3), "own Spirit gets +1/+1");
    assert_eq!(
        pt(&engine, my_bears),
        (2, 2),
        "non-Spirit creature unaffected"
    );
    assert_eq!(
        pt(&engine, their_kami),
        (2, 2),
        "opponent's Spirit is not pumped"
    );

    // 2. Arcane cost reduction: with 2 Forests floating, Kodama's Reach ({2}{G} Arcane)
    // is castable for {1}{G}, whereas non-Arcane {3} Long-Forgotten Gohei is NOT.
    let reach = in_hand(&engine, p0, kodama_s_reach()).unwrap();
    let second_gohei = in_hand(&engine, p0, long_forgotten_gohei()).unwrap();
    tap_all_mana(&mut engine, p0);
    let castable = &priority_offer(&engine).castable;
    assert!(
        castable.contains(&reach),
        "Arcane spell {{2}}{{G}} discounted to {{1}}{{G}}"
    );
    assert!(
        !castable.contains(&second_gohei),
        "non-Arcane {{3}} spell is NOT discounted"
    );
    cast_with_floating(&mut engine, p0, kodama_s_reach());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 3. Opponent's Arcane spell is NOT discounted by your Gohei (2 Forests cannot pay {2}{G}).
    reach_their_main_phase(&mut engine, p1);
    let their_reach = in_hand(&engine, p1, kodama_s_reach()).unwrap();
    tap_all_mana(&mut engine, p1);
    assert!(
        !priority_offer(&engine).castable.contains(&their_reach),
        "Gohei discounts only Arcane spells YOU cast"
    );
}
