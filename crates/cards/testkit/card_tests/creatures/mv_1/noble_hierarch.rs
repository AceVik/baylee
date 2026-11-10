//! `cards/creatures/mv_1/noble_hierarch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Noble Hierarch` prints `Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)` and `{{T}}: Add {{G}}, {{W}}, or {{U}}.`
///
/// Activating its printed mana ability prompts via `Pending::ChooseColor` with `ManaColor::Green`, `ManaColor::White`, and `ManaColor::Blue`, producing the chosen mana and tapping `Noble Hierarch`.
/// When a controlled `young_wolf()` attacks alone, exalted makes it 2/2 for the turn.
#[test]
fn noble_hierarch_produces_mana_choice_and_exalts_the_lone_attacker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[noble_hierarch(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hierarch = on_battlefield(&engine, p0, noble_hierarch()).expect("hierarch on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, hierarch), (0, 1));
    assert_eq!(pt(&engine, wolf), (1, 1));

    activate(&mut engine, p0, noble_hierarch(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![ManaColor::Green, ManaColor::White, ManaColor::Blue],
        "hierarch offers Green, White, or Blue"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
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
