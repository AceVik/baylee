//! `cards/creatures/mv_4/lhurgoyf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

fn lhurgoyf() -> CardIndex {
    card_index("0fe76004-b2f4-4b11-abce-75fd76f68b4d")
}

fn cast(engine: &mut Engine<RegistryLookup>, card: CardIndex) {
    pass_until(engine, |e| {
        e.state().turn.active == P0
            && matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain)
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    for color in ManaColor::ALL {
        engine.state.players[0].mana_pool.add(color, 10);
    }
    engine.refresh_offer();
    cast_with_floating(engine, P0, card);
}

fn resolve_at(engine: &mut Engine<RegistryLookup>, card: CardIndex, target: ObjectId) {
    cast(engine, card);
    aim_at(engine, P0, target);
    pass_until(engine, stack_is_empty);
}

/// Only creature cards in graveyards count: live creatures and a creature in
/// hand do not, while creatures killed on either side immediately increase it.
#[test]
fn lhurgoyf_grows_for_both_players_dead_creatures_but_not_the_removal_spells() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(
            0,
            &[
                lhurgoyf(),
                llanowar_elves(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let mine = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&engine, P1, llanowar_elves()).unwrap();

    cast(&mut engine, lhurgoyf());
    pass_until(&mut engine, stack_is_empty);
    let goyf = on_battlefield(&engine, P0, lhurgoyf()).expect("Lhurgoyf resolved");
    assert_eq!(pt(&engine, goyf), (0, 1), "no creature card has died");

    resolve_at(&mut engine, lightning_bolt(), theirs);
    assert!(in_graveyard(&engine, P1, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, P0, lightning_bolt()).is_some());
    assert_eq!(pt(&engine, goyf), (1, 2), "the instant does not count");

    resolve_at(&mut engine, lightning_bolt(), mine);
    assert!(in_graveyard(&engine, P0, llanowar_elves()).is_some());
    assert_eq!(pt(&engine, goyf), (2, 3), "both graveyards contribute");
}

/// "All graveyards" includes every seat at a multiplayer table. Exiling one
/// graveyard removes its creature cards from the live count.
#[test]
fn lhurgoyf_counts_every_graveyard_and_shrinks_when_one_is_exiled() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .hand(0, &[lhurgoyf(), llanowar_elves(), forest(), bojuka_bog()])
        .hand(1, &[llanowar_elves(), llanowar_elves(), island()])
        .hand(
            2,
            &[
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    for (seat, creatures, land) in [(P0, 1, forest()), (P1, 2, island()), (P2, 3, island())] {
        for _ in 0..creatures {
            hand_to_graveyard(&mut engine, seat, llanowar_elves());
        }
        hand_to_graveyard(&mut engine, seat, land);
    }

    cast(&mut engine, lhurgoyf());
    pass_until(&mut engine, stack_is_empty);
    let goyf = on_battlefield(&engine, P0, lhurgoyf()).unwrap();
    assert_eq!(pt(&engine, goyf), (6, 7), "lands in the yards are excluded");

    play_land(&mut engine, P0, bojuka_bog());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P2],
            },
        )
        .expect("the Bog targets the third player's graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P2))
            .is_empty()
    );
    assert_eq!(
        pt(&engine, goyf),
        (3, 4),
        "exiled creatures no longer count"
    );
}

/// The size definition works in hand and on the stack, survives a bounce,
/// and counts Lhurgoyf itself once it is a creature card in a graveyard.
#[test]
fn lhurgoyf_defines_its_size_outside_the_battlefield_and_counts_itself_in_the_graveyard() {
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[lhurgoyf(), llanowar_elves(), unsummon(), lightning_bolt()],
        )
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    hand_to_graveyard(&mut engine, P0, llanowar_elves());
    hand_to_graveyard(&mut engine, P1, llanowar_elves());
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    let goyf = in_hand(&engine, P0, lhurgoyf()).unwrap();
    assert_eq!(pt(&engine, goyf), (2, 3), "the CDA functions in hand");

    cast(&mut engine, lhurgoyf());
    assert_eq!(on_stack(&engine, lhurgoyf()), Some(goyf));
    assert_eq!(pt(&engine, goyf), (2, 3), "the CDA functions on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, goyf), (2, 3));

    resolve_at(&mut engine, unsummon(), goyf);
    assert_eq!(in_hand(&engine, P0, lhurgoyf()), Some(goyf));
    assert_eq!(pt(&engine, goyf), (2, 3), "returning to hand keeps the CDA");
    cast(&mut engine, lhurgoyf());
    pass_until(&mut engine, stack_is_empty);
    resolve_at(&mut engine, lightning_bolt(), goyf);
    assert_eq!(in_graveyard(&engine, P0, lhurgoyf()), Some(goyf));
    assert_eq!(
        pt(&engine, goyf),
        (3, 4),
        "it counts itself in the graveyard"
    );
}
