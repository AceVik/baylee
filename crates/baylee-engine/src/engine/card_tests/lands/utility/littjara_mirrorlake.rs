//! `cards/lands/utility/littjara_mirrorlake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Littjara Mirrorlake prints `This land enters tapped.`, `{{T}}: Add {{U}}.`, and `{{2}}{{G}}{{G}}{{U}}, {{T}}, Sacrifice this land: Create a token that's a copy of target creature you control, except it enters with an additional +1/+1 counter on it. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, the copy token's additional +1/+1 counter is omitted because `Effect::CreateTokenCopyOf` carries no modifier list.
/// Playing Littjara Mirrorlake enters tapped. In the following turn, with `{{2}}{{G}}{{G}}{{U}}` floating from basic lands,
/// ability 1 targets only a controlled creature (`llanowar_elves()`) and not an opponent's creature.
/// Upon resolution, Littjara Mirrorlake is sacrificed and a copy token enters
/// as a 1/1 without a `CounterKind::Plus` counter.
#[test]
fn littjara_mirrorlake_enters_tapped_and_creates_creature_copy() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[littjara_mirrorlake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lake = play_land(&mut engine, p0, littjara_mirrorlake());
    assert!(
        entered_tapped(&engine, lake),
        "littjara mirrorlake enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, lake));

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("controlled elf");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");

    // Float {{2}}{{G}}{{G}}{{U}} while keeping Littjara Mirrorlake untapped —
    // six, because the Elves tap for their own {{G}} beside the lands.
    tap_mana_except(&mut engine, p0, lake);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);

    activate(&mut engine, p0, littjara_mirrorlake(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&opp_elf),
        "opponent's creature is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_elf],
                players: vec![],
            },
        )
        .unwrap();

    assert!(
        in_graveyard(&engine, p0, littjara_mirrorlake()).is_some(),
        "littjara mirrorlake sacrificed as cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}}{{G}}{{G}}{{U}} out of six leaves the Elves' own mana standing"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one copy token created");
    assert_eq!(pt(&engine, tokens[0]), (1, 1), "token is a 1/1 elf");
    assert_eq!(
        counters_on(
            &engine,
            tokens[0],
            CounterKind::Plus {
                power: 1,
                toughness: 1,
            }
        ),
        0,
        "additional +1/+1 counter omitted under `Coverage::Partial`"
    );
}
