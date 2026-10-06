//! `cards/lands/manlands/restless_cottage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Cottage: "This land enters tapped." / "{T}: Add {B} or {G}." / "{2}{B}{G}: This land becomes a 4/4 black and green Horror creature until end of turn. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{2}{B}{G}` after untapping animates it into a 4/4 Horror creature.
#[test]
fn restless_cottage_animates_into_horror() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(312, forest())
        .battlefield(0, &[swamp(), swamp(), forest(), forest()])
        .hand(0, &[restless_cottage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_cottage());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_cottage(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (4, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Cottage: "Whenever this land attacks, create a Food token and
/// exile up to one target card from a graveyard."
#[test]
fn restless_cottage_attack_makes_a_food_and_exiles_a_card_from_a_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4101, forest())
        .battlefield(
            0,
            &[restless_cottage(), swamp(), swamp(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    seed_graveyard(&mut engine, p1, 1);
    let victim = engine.state().zones.list(ZoneLocation::Graveyard(p1))[0];
    animate_and_attack(&mut engine, restless_cottage(), &[]);
    aim_trigger_at(&mut engine, victim);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, p0).len(), 1, "one Food token");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "the targeted card left the graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&victim)
    );
}
