//! `cards/creatures/mv_1/lake_town_lookout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Lake-town Lookout` prints `When this creature dies, recruit. (Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token.)`
///
/// Marked `Coverage::Partial`, its death trigger fires `Trigger::Dies` with `Effect::draw(1)` and `Effect::DiscardForPlayers`.
/// When destroyed by `heroes_downfall()`, the controller draws a card, answers `Pending::ChooseCards` to discard a card via `PlayerAction::ChooseObjects`, and under `Coverage::Partial` no Soldier creature token is created.
#[test]
fn lake_town_lookout_dies_draws_and_discards_without_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lake_town_lookout(), swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lookout = on_battlefield(&engine, p0, lake_town_lookout()).expect("lookout on battlefield");
    assert_eq!(pt(&engine, lookout), (1, 1));
    let initial_library_size = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    let downfall = in_hand(&engine, p0, heroes_downfall()).expect("downfall in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: downfall })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for downfall, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&lookout));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![lookout],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::Generic);
    assert!(
        in_graveyard(&engine, p0, lake_town_lookout()).is_some(),
        "lookout is in graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_library_size - 1,
        "recruit drew one card before prompting for discard"
    );

    let to_discard = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![to_discard],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "under `Coverage::Partial` recruit token creation is omitted"
    );
}
