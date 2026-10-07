//! `cards/lands/utility/mirrorpool.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mirrorpool: "This land enters tapped." / "{T}: Add {C}." / "{2}{C}, {T}, Sacrifice this land..." / "{4}{C}, {T}, Sacrifice this land..."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its copy abilities are fully realized.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for colorless mana.
#[test]
fn mirrorpool_enters_tapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(338, forest()).hand(0, &[mirrorpool()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrorpool());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, mirrorpool(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Mirrorpool: "{2}{C}, {T}, Sacrifice this land: Copy target instant or
/// sorcery spell you control." A Bolt aimed at the opponent is copied; both
/// resolve.
#[test]
fn mirrorpool_copies_a_spell_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4207, mountain())
        .battlefield(0, &[mirrorpool(), mountain(), mountain(), sol_ring()])
        .hand(0, &[lightning_bolt()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(mirrorpool()));
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the Bolt aims at the opponent");
    let bolt = on_stack(&engine, lightning_bolt()).expect("the Bolt is on the stack");
    activate(&mut engine, p0, mirrorpool(), 1);
    answer_target(&mut engine, bolt, &[]);
    for _ in 0..30 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                if stack_is_empty(&engine) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets { player, min: 0, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected on the way to resolution: {other:?}"),
        }
    }
    assert_eq!(
        engine.state().players[1].life,
        14,
        "the Bolt and its copy each dealt 3"
    );
    assert!(
        on_battlefield(&engine, p0, mirrorpool()).is_none(),
        "sacrificed"
    );
}

/// Mirrorpool: "{4}{C}, {T}, Sacrifice this land: Create a token that's a
/// copy of target creature you control."
#[test]
fn mirrorpool_copies_a_creature_you_control_as_a_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4208, mountain())
        .battlefield(
            0,
            &[
                mirrorpool(),
                mountain(),
                mountain(),
                mountain(),
                sol_ring(),
                viashino_grappler(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lizard = on_battlefield(&engine, p0, viashino_grappler()).expect("the Lizard");
    tap_all_mana_but(&mut engine, p0, Some(mirrorpool()));
    activate(&mut engine, p0, mirrorpool(), 2);
    answer_target(&mut engine, lizard, &[]);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token");
    assert_eq!(pt(&engine, tokens[0]), (3, 1), "a copy of the Lizard");
    assert!(on_battlefield(&engine, p0, mirrorpool()).is_none());
}
