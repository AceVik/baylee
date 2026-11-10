//! `cards/creatures/mv_1/ignoble_hierarch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ignoble Hierarch` prints `Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)` and `{{T}}: Add {{B}}, {{R}}, or {{G}}.`
///
/// Activating its printed mana ability prompts via `Pending::ChooseColor` with `ManaColor::Black`, `ManaColor::Red`, and `ManaColor::Green`, producing the chosen mana and tapping `Ignoble Hierarch`.
/// When a controlled `young_wolf()` attacks alone, exalted makes it 2/2 for the turn.
#[test]
fn ignoble_hierarch_produces_mana_choice_and_exalts_the_lone_attacker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ignoble_hierarch(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hierarch =
        on_battlefield(&engine, p0, ignoble_hierarch()).expect("hierarch on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, hierarch), (0, 1));
    assert_eq!(pt(&engine, wolf), (1, 1));

    activate(&mut engine, p0, ignoble_hierarch(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red, ManaColor::Green],
        "hierarch offers Black, Red, or Green"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, hierarch));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wolf, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wolf),
        (2, 2),
        "exalted gives the lone attacker +1/+1"
    );
}

#[test]
fn exalted_stacks_in_multiplayer_but_not_when_attacking_two_opponents() {
    let me = PlayerId::new(0);
    for alone in [true, false] {
        let mut engine = Duel::table(928, forest(), 3)
            .battlefield(
                0,
                &[
                    ignoble_hierarch(),
                    noble_hierarch(),
                    cathedral_of_war(),
                    young_wolf(),
                    llanowar_elves(),
                ],
            )
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, me));
        let wolf = on_battlefield(&engine, me, young_wolf()).unwrap();
        let elf = on_battlefield(&engine, me, llanowar_elves()).unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let mut attackers = vec![(wolf, Defender::Player(PlayerId::new(1)))];
        if !alone {
            attackers.push((elf, Defender::Player(PlayerId::new(2))));
        }
        engine
            .apply(me, PlayerAction::DeclareAttackers { attackers })
            .unwrap();
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Stack).len(),
            if alone { 3 } else { 0 }
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(pt(&engine, wolf), if alone { (4, 4) } else { (1, 1) });
        assert_eq!(pt(&engine, elf), (1, 1));
        pass_until(&mut engine, |e| e.state().turn.active != me);
        assert_eq!(
            pt(&engine, wolf),
            (1, 1),
            "all three pumps expire at end of turn"
        );
    }
}
