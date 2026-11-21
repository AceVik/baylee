//! `cards/creatures/mv_6/treva_the_renewer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn treva_the_renewer() -> CardIndex {
    card_index("72524f3c-674b-4ebf-9526-18737b354010")
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

fn shu_general() -> CardIndex {
    card_index("0fef6729-b044-4daf-834a-ac8c9711ad8a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Treva, the Renewer (72524f3c-674b-4ebf-9526-18737b354010) — `{3}{G}{W}{U}` 6/6
/// Legendary Creature — Dragon:
/// - Flying
/// - Whenever Treva deals combat damage to a player, you may pay `{2}{W}`. If you do,
///   choose a color, then you gain 1 life for each permanent of that color.
#[test]
fn treva_the_renewer_pays_two_white_to_gain_one_life_per_permanent_of_chosen_color() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                forest(),
                plains(),
                island(),
                plains(),
                plains(),
                plains(),
                glorious_anthem(),
            ],
        )
        .battlefield(1, &[shu_general(), grizzly_bears(), plains()])
        .hand(0, &[treva_the_renewer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, treva_the_renewer());
    pass_until(&mut engine, stack_is_empty);

    let treva = on_battlefield(&engine, P0, treva_the_renewer()).expect("Treva entered");
    // Glorious Anthem (+1/+1) makes Treva 7/7; base is 6/6.
    assert_eq!(pt(&engine, treva), (7, 7));
    assert!(keywords(&engine, treva).contains(KeywordSet::FLYING));

    // White permanents on battlefield: Treva (GWU), Glorious Anthem (W), P1's Shu General (W) = 3.
    // Lands (Plains) are colorless and P1's Grizzly Bears is green.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let life_before = engine.state().players[0].life;
    let _ = attack_and_collect_blocks(&mut engine, treva, P1);

    pass_until(&mut engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().players[0].life,
        life_before + 3,
        "gains 1 life for each of the 3 white permanents across both sides"
    );
}
