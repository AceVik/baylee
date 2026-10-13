//! `cards/lands/manlands/restless_spire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Spire: "This land enters tapped." / "{T}: Add {U} or {R}." / "{U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature..."
/// Under `Coverage::Partial`, the granted first strike during your turn is omitted because no modifier grants turn-conditional keywords.
/// Playing this land causes it to enter tapped, and paying `{U}{R}` after untapping animates it into a 2/1 Elemental creature.
#[test]
fn restless_spire_animates_into_elemental() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(316, forest())
        .battlefield(0, &[island(), mountain()])
        .hand(0, &[restless_spire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_spire());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_spire(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 1));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

#[test]
fn animated_spire_first_strike_follows_its_current_controllers_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::RESTLESS_SPIRE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::RESTLESS_SPIRE).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.players[0].mana_pool.add(ManaColor::Blue, 1);
    state.players[0].mana_pool.add(ManaColor::Red, 1);
    e.refresh_offer();
    activate(&mut e, p0, index::RESTLESS_SPIRE, 1);
    pass_until(&mut e, stack_is_empty);
    assert!(keywords(&e, land).contains(KeywordSet::FIRST_STRIKE));
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(land).unwrap().set_controller(p1);
    state.invalidate_projections();
    state.refresh_characteristics();
    assert!(!keywords(&e, land).contains(KeywordSet::FIRST_STRIKE));
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(land).unwrap().set_controller(p0);
    state.invalidate_projections();
    state.refresh_characteristics();
    assert!(keywords(&e, land).contains(KeywordSet::FIRST_STRIKE));
    reach_their_main_phase(&mut e, p1);
    assert!(!keywords(&e, land).contains(KeywordSet::FIRST_STRIKE));
}

/// "Whenever this land attacks, scry 1." It scries for the land's controller
/// when the land is declared as an attacker.
#[test]
fn restless_spire_scries_one_when_it_attacks() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::RESTLESS_SPIRE])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::RESTLESS_SPIRE).unwrap();
    let pool = &mut e.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::Blue, 1);
    pool.add(ManaColor::Red, 1);
    e.refresh_offer();
    activate(&mut e, p0, index::RESTLESS_SPIRE, 1);
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(land, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0, "its controller scries");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(piles, scry_piles(1));
    assert_eq!(cards.len(), 1, "scry 1");
}
