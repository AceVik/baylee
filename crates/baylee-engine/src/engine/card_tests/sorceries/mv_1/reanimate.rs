//! `cards/sorceries/mv_1/reanimate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// "You lose life equal to its mana value": the card's mana value where
/// Reanimate expected it, in the graveyard, since the resolution has moved
/// it by then (CR 608.2h). A Clone reanimated as a copy of Serra Angel is a
/// five on the battlefield and was a four in the graveyard, and its
/// controller loses four. `Amount::TargetCmc` used to read the creature that
/// had entered, and took five.
#[test]
fn reanimate_takes_the_mana_value_the_card_had_in_the_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[index::REANIMATE, index::CLONE])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let clone = in_hand(&engine, p0, index::CLONE).expect("Clone in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            clone,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Setup,
        )
        .expect("Clone goes to the graveyard");
    engine.refresh_offer();
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("Serra Angel");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::REANIMATE);
    aim_at(&mut engine, p0, clone);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .expect("Clone enters as a copy of Serra Angel");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(clone)
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| o.characteristics().mana_value()),
        Some(5),
        "the reanimated Clone is a Serra Angel, mana value five"
    );
    assert_eq!(
        engine.state().players[0].life,
        16,
        "Clone's own four, as it was in the graveyard"
    );
}
