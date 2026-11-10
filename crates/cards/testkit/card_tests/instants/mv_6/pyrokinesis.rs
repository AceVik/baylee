//! `cards/instants/mv_6/pyrokinesis.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn pyrokinesis() -> CardIndex {
    card_index("d001febc-d511-4e65-a631-91dc9415056b")
}

/// Pyrokinesis ({4}{R}{R} Instant):
/// - "You may exile a red card from your hand rather than pay this spell's mana cost."
/// - "Pyrokinesis deals 4 damage divided as you choose among any number of target creatures."
#[test]
fn pyrokinesis_pitch_casts_by_exiling_red_card_and_divides_four_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(1, &[grizzly_bears(), HILL_GIANT])
        .hand(0, &[pyrokinesis(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pyro = in_hand(&engine, p0, pyrokinesis()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    // Cast Pyrokinesis with no mana by exiling Lightning Bolt from hand.
    engine
        .apply(p0, PlayerAction::CastSpell { card: pyro })
        .unwrap();
    // "Target creatures": a player is no target.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    // Divide 4 damage: 2 to Grizzly Bears (leaving 2 for Hill Giant).
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    // Pay pitch cost by exiling Lightning Bolt.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert_eq!(engine.state().object(bolt).unwrap().zone, Zone::Exile);
}

/// Pyrokinesis — "You may exile a red card from your hand rather than pay
/// this spell's mana cost.": with no mana at all, a red card in hand makes it
/// castable and a card of another colour does not.
#[test]
fn pyrokinesis_is_pitched_only_for_a_red_card() {
    let p0 = PlayerId::new(0);
    let castable_with = |other: CardIndex| {
        let mut engine = Duel::new(SEED, mountain())
            .hand(0, &[pyrokinesis(), other])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let pyro = in_hand(&engine, p0, pyrokinesis()).unwrap();
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("priority, got {:?}", engine.pending())
        };
        legal.castable.contains(&pyro)
    };
    assert!(castable_with(lightning_bolt()), "a red card pays for it");
    assert!(!castable_with(grizzly_bears()), "a green one does not");
}
