//! `cards/creatures/mv_4/world_shaper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// World Shaper: the attack trigger.
///
/// "…you may mill three cards" is a `MayDo`, so the question is asked and
/// answering it is the test: three cards leave the library for the graveyard
/// only because a seat said yes.
#[test]
fn world_shaper_mills_three_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(389, forest())
        .battlefield(0, &[world_shaper()])
        .start();
    keep_mulligans(&mut engine);

    let shaper = on_battlefield(&engine, p0, world_shaper()).expect("the Shaper is seated");
    let before = library_size(&engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shaper, Defender::Player(PlayerId::new(1)))],
            },
        )
        .expect("the Shaper attacks");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        before - 3,
        "three cards milled off the attack trigger"
    );
}

/// World Shaper: "When this creature dies, return all land cards from your
/// graveyard to the battlefield tapped."
///
/// Two Forests and an Elf lie in its controller's graveyard and a Plains in
/// the opponent's. The opponent's Vindicate kills the Shaper: both Forests
/// come back tapped under the Shaper's controller, and the Elf, the Shaper
/// itself and the other graveyard's Plains stay where they are.
#[test]
fn world_shaper_dying_returns_every_land_card_from_its_graveyard_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(390, forest())
        .battlefield(0, &[world_shaper()])
        .hand(0, &[forest(), forest(), llanowar_elves()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate(), plains()])
        .start();
    keep_mulligans(&mut engine);
    let forests = [
        hand_to_graveyard(&mut engine, p0, forest()),
        hand_to_graveyard(&mut engine, p0, forest()),
    ];
    let elf = hand_to_graveyard(&mut engine, p0, llanowar_elves());
    let their_plains = hand_to_graveyard(&mut engine, p1, plains());

    reach_their_main_phase(&mut engine, p1);
    let shaper = on_battlefield(&engine, p0, world_shaper()).expect("the Shaper is seated");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![shaper],
            },
        )
        .expect("Vindicate targets the Shaper");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, world_shaper()).is_some(),
        "it died"
    );
    let lands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(lands.len(), 2, "both Forests came back: {forests:?}");
    assert!(
        lands.iter().all(|land| is_tapped(&engine, *land)),
        "\"to the battlefield tapped\""
    );
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(Zone::Graveyard),
        "only land cards return"
    );
    assert_eq!(
        engine.state().object(their_plains).map(|o| o.zone),
        Some(Zone::Graveyard),
        "\"your graveyard\": the opponent's Plains stays"
    );
}

/// World Shaper's dies trigger again, from a Lightning Bolt and with the
/// cards milled rather than discarded.
///
/// A Lightning Bolt kills it. The two Forests in its controller's graveyard
/// come back tapped and under their control; the Llanowar Elves there is no
/// land card and stays, and the Forest in the opponent's graveyard is not
/// "your graveyard" and stays too.
#[test]
fn world_shaper_brings_back_every_land_in_your_graveyard_tapped_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(390, forest())
        .battlefield(0, &[world_shaper(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let shaper = on_battlefield(&engine, p0, world_shaper()).expect("the Shaper is seated");
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            elves,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("into the graveyard");
    seed_graveyard(&mut engine, p0, 2);
    seed_graveyard(&mut engine, p1, 1);
    let buried: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(buried.len(), 2, "two Forests milled into p0's graveyard");
    let theirs = in_graveyard(&engine, p1, forest()).expect("a Forest in p1's graveyard");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).expect("the Bolt is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .expect("a Mountain pays for the Bolt");
    aim_at(&mut engine, p1, shaper);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, world_shaper()).is_some(),
        "the Shaper died"
    );
    for land in buried {
        let object = engine
            .state()
            .object(land)
            .expect("the Forest still exists");
        assert_eq!(object.zone, Zone::Battlefield, "a land card came back");
        assert_eq!(object.controller, p0, "under its owner's control");
        assert!(is_tapped(&engine, land), "tapped");
    }
    assert_eq!(
        engine.state().object(elves).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the Elves are no land card"
    );
    assert_eq!(
        engine.state().object(theirs).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the opponent's graveyard is not yours"
    );
}
