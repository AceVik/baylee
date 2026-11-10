//! `cards/creatures/mv_7/shivan_hellkite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Shivan Hellkite` is a 5/5 Dragon costing `{5}{R}{R}` under `Coverage::Implemented` with flying.
/// It prints "{1}{R}: This creature deals 1 damage to any target."
/// When activated off floating mana, its ability targets an opponent and resolves, dealing 1 damage
/// to that player and reducing their life total from 20 to 19.
#[test]
fn shivan_hellkite_activates_to_ping_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shivan_hellkite(), mountain(), mountain()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let dragon =
        on_battlefield(&engine, p0, shivan_hellkite()).expect("Shivan Hellkite is present");
    assert_eq!(pt(&engine, dragon), (5, 5), "printed body is 5/5");
    assert!(
        keywords(&engine, dragon).contains(KeywordSet::FLYING),
        "Shivan Hellkite has flying"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two red mana floated to cover {{1}}{{R}}"
    );

    activate(&mut engine, p0, shivan_hellkite(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Shivan Hellkite, got {:?}",
            engine.pending()
        );
    };
    assert!(
        player_options.contains(&p1),
        "opponent is offered as legal target: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p1],
            },
        )
        .expect("targeting opponent is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "Shivan Hellkite dealt 1 damage to opponent"
    );
}
