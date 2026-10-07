//! `cards/creatures/mv_3/murderous_rider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Murderous Rider` prints `Lifelink`, `When this creature dies, put it on the bottom of its owner's library.`, and `Destroy target creature or planeswalker. You lose 2 life. (Then exile this card. You may cast the creature later from exile.)`
///
/// The dies trigger is played in the two tests below this one. Swift End is cast via
/// `Pending::ChooseCastMode` with `CastModeKind::Face(1)`: it destroys the opponent's
/// `llanowar_elves()`, its caster loses 2 life, and the card goes on its adventure — exiled
/// with the rider that lets the creature be cast from there (CR 715.3d), not into the
/// graveyard. The Rider is then cast out of that exile and lands as a 2/3 with lifelink.
#[test]
fn murderous_rider_swift_end_goes_on_an_adventure_and_the_rider_follows_from_exile() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[murderous_rider()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Swamps floating: three for Swift End, three for the Rider after it,
    // all inside one main phase so the pool is not emptied in between.
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, murderous_rider()).expect("rider in hand");
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode prompt, got {:?}", engine.pending());
    };
    let adventure_slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Face(1)))
        .expect("Swift End adventure face");
    engine
        .apply(p0, PlayerAction::ChooseMode(adventure_slot))
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");
    assert!(
        options.contains(&elf),
        "opponent creature is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "target creature was destroyed"
    );
    assert_eq!(engine.state().players[0].life, 18, "caster lost 2 life");
    assert!(
        in_graveyard(&engine, p0, murderous_rider()).is_none(),
        "an adventure does not resolve to the graveyard"
    );
    let exiled = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == murderous_rider()))
        })
        .expect("the card went on its adventure (CR 715.3d)");
    assert!(
        engine
            .state()
            .object(exiled)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Adventure)),
        "wearing the rider that says it may be cast from there"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&exiled),
        "the creature is on offer out of the exile its adventure made"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .expect("the offer is honoured");
    pass_until(&mut engine, stack_is_empty);

    let rider = on_battlefield(&engine, p0, murderous_rider()).expect("the Rider landed");
    assert_eq!(pt(&engine, rider), (2, 3));
    assert!(
        keywords(&engine, rider).contains(KeywordSet::LIFELINK),
        "front face has lifelink"
    );
}

/// Murderous Rider: "When this creature dies, put it on the bottom of its
/// owner's library." Bolted, it goes to the graveyard, the trigger resolves,
/// and the card is the bottom card of its owner's library — not the top, and
/// not in the graveyard.
#[test]
fn murderous_rider_dies_and_goes_to_the_bottom_of_its_owners_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(388, island())
        .battlefield(0, &[murderous_rider()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let rider = on_battlefield(&engine, p0, murderous_rider()).expect("the Rider is seated");

    bolt_to_death(&mut engine, p1, rider);
    assert!(!stack_is_empty(&engine), "the dies trigger is waiting");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, murderous_rider()).is_none(),
        "the Rider left the graveyard"
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    assert!(
        engine_card_is(&engine, library[0], murderous_rider()),
        "and is the bottom card of its owner's library"
    );
    assert!(
        !engine_card_is(&engine, library[library.len() - 1], murderous_rider()),
        "not the top one"
    );
}

/// Veteran Bodyguard counter-check, with a real lifelink attacker: an
/// unblocked Murderous Rider's damage is redirected onto the untapped
/// Bodyguard exactly as any other unblocked creature's would be, and its
/// controller still gains the life lifelink promises — the keyword reads
/// off the source, not off where the damage actually lands.
#[test]
fn an_unblocked_lifelinkers_redirected_damage_still_gains_its_controller_the_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[murderous_rider()])
        .battlefield(1, &[veteran_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rider = on_battlefield(&engine, p0, murderous_rider()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    assert!(keywords(&engine, rider).contains(KeywordSet::LIFELINK));
    assert!(!is_tapped(&engine, guard), "untapped to start");
    let before_life0 = engine.state().players[0].life;
    let before_life1 = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rider, Defender::Player(p1))],
            },
        )
        .expect("the Rider attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks: the Rider is unblocked");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        2,
        "the Rider's power, redirected onto the Bodyguard"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life1,
        "not a point off the Bodyguard's controller"
    );
    assert_eq!(
        engine.state().players[0].life,
        before_life0 + 2,
        "lifelink still pays the Rider's controller, wherever the damage landed"
    );
}
