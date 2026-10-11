//! `cards/lands/utility/forbidden_orchard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forbidden Orchard: "{T}: Add one mana of any color." / "Whenever you tap this land for mana, target opponent creates a 1/1 colorless Spirit creature token."
/// Activating ability 0 prompts for a color, produces that mana, and taps the land.
#[test]
fn forbidden_orchard_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[forbidden_orchard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let orchard = on_battlefield(&engine, p0, forbidden_orchard()).expect("Orchard deployed");
    activate(&mut engine, p0, forbidden_orchard(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, orchard));
}

#[test]
fn orchard_targets_one_opponent_and_uses_that_players_token_replacements() {
    use baylee_core::generated::index;
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[forbidden_orchard(), index::DOUBLING_SEASON])
        .battlefield(2, &[index::DOUBLING_SEASON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, forbidden_orchard(), 0);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Green),
        1
    );
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p2],
        },
    )
    .unwrap();
    assert!(
        !stack_is_empty(&e),
        "the mana is immediate, the targeted trigger is not"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(tokens_of(&e, p0).len(), 0);
    assert_eq!(tokens_of(&e, p1).len(), 0);
    assert_eq!(tokens_of(&e, p2).len(), 2);
    for token in tokens_of(&e, p2) {
        assert_eq!(pt(&e, token), (1, 1));
        assert!(
            e.state()
                .object(token)
                .unwrap()
                .characteristics()
                .colors
                .is_empty()
        );
    }
}
