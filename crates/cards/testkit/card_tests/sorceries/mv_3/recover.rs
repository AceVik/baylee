// Oracle checklist: Return target creature card from your graveyard to your hand.; Draw a card.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("f4361602-63da-430b-adbb-36d74e987aa6")
}

// Only a creature card in caster's graveyard may return; drawing is additional.
#[test]
fn returns_own_creature_card_and_draws_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), gray_ogre(), quiet_artifact()],
        )
        .battlefield(1, &[wild_elephant()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let own = on_battlefield(&game, p0, gray_ogre()).unwrap();
    let own_art = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    let theirs = on_battlefield(&game, p1, wild_elephant()).unwrap();
    bury(&mut game, &[own, own_art, theirs]);
    game.refresh_offer();
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut game, p0, subject());
    let own_dead = in_graveyard(&game, p0, gray_ogre()).unwrap();
    let art_dead = in_graveyard(&game, p0, quiet_artifact()).unwrap();
    let their_dead = in_graveyard(&game, p1, wild_elephant()).unwrap();
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("graveyard target required")
    };
    assert!(options.contains(&own_dead));
    assert!(!options.contains(&art_dead));
    assert!(!options.contains(&their_dead));
    aim_at(&mut game, p0, own_dead);
    pass_until(&mut game, stack_is_empty);
    assert!(in_hand(&game, p0, gray_ogre()).is_some());
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert!(in_graveyard(&game, p0, subject()).is_some());
}
