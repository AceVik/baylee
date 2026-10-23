//! `cards/instants/mv_1/ebony_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ebony_charm() -> CardIndex {
    card_index("2f3237fe-9e07-4aad-8bc8-7ef4b510dd36")
}

/// Ebony Charm (2f3237fe-9e07-4aad-8bc8-7ef4b510dd36) — {B} Instant:
/// "Choose one —
/// • Target opponent loses 1 life and you gain 1 life.
/// • Exile up to three target cards from a single graveyard.
/// • Target creature gains fear until end of turn."
#[test]
fn ebony_charm_drains_one_life_or_grants_fear() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), GRIZZLY_BEARS])
        .hand(0, &[ebony_charm(), ebony_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    // Mode 0: Target opponent loses 1 life and you gain 1 life.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ebony_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot0)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 21);

    // Mode 2: Target creature gains fear until end of turn.
    cast_with_floating(&mut engine, P0, ebony_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot2 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot2)).unwrap();
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FEAR)
    );
}

/// Mode 2 as the card plays it (the card is a Partial): "up to three target
/// cards from a single graveyard" is one card here, because a spell cannot
/// be held to a single graveyard. Both graveyards are on the menu, a second
/// target is refused, and the one card named is exiled while the rest of
/// both graveyards stay.
#[test]
fn ebony_charm_exiles_at_most_one_card_from_a_graveyard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[ebony_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    seed_graveyard(&mut engine, P0, 1);
    seed_graveyard(&mut engine, P1, 4);
    let mine = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P0))
        .clone();
    let theirs = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P1))
        .clone();
    assert_eq!((mine.len(), theirs.len()), (1, 4));

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ebony_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot1 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot1)).unwrap();
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the targets, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1), "at most one target card");
    assert_eq!(options.len(), 5, "every graveyard card is a legal target");
    assert!(options.contains(&mine[0]) && theirs.iter().all(|c| options.contains(c)));

    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: theirs[..2].to_vec(),
                },
            )
            .is_err(),
        "a second target is refused"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(theirs[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Exile)
    );
    for &card in theirs[1..].iter().chain(&mine) {
        assert_eq!(
            engine.state().object(card).map(|o| o.zone),
            Some(crate::zone::Zone::Graveyard),
            "only the named card is exiled"
        );
    }
}
