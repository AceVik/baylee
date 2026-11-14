//! `cards/2001/src/ody/fervent_denial.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fervent_denial() -> CardIndex {
    card_index("2e56bc34-1e35-4b95-9525-cb58ac8ba060")
}

fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(slot))
            .unwrap();
    }
}

/// Counters target spell from hand for {3}{U}{U}; refuses Flashback on 6
/// mana; counters a second spell via Flashback {5}{U}{U} and exiles itself.
#[test]
fn fervent_denial_counters_from_hand_and_flashback_before_exiling() {
    // 11 Islands: 5 spent on {3}{U}{U} from hand leaves 6 untapped to verify
    // the 6-mana Flashback {5}{U}{U} refusal; all 11 untap on the next cycle.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 11])
        .battlefield(1, &[forest(); 6])
        .hand(0, &[fervent_denial()])
        .hand(1, &[grizzly_bears(), grizzly_bears(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, me()));

    reach_their_main_phase(&mut engine, them());
    cast_from_hand(&mut engine, them(), grizzly_bears());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    let bears1 = on_stack(&engine, grizzly_bears()).expect("first Bears on stack");
    cast_from_hand(&mut engine, me(), fervent_denial());
    aim_at(&mut engine, me(), bears1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, them(), grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, them(), grizzly_bears()).is_some());
    let denial = in_graveyard(&engine, me(), fervent_denial()).expect("in graveyard");

    // Still in P1's main phase (6 Islands untapped, 4 Forests left): P1 casts Bears #2.
    cast_from_hand(&mut engine, them(), grizzly_bears());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    tap_all_mana(&mut engine, me());
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&denial),
        "6 mana cannot pay {{5}}{{U}}{{U}}"
    );
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card: denial })
            .is_err()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, them(), grizzly_bears()).is_some());

    // Next cycle: P0's 11 Islands untap; P1 casts Bears #3 -> Flashback {5}{U}{U} counters it.
    reach_their_main_phase(&mut engine, me());
    reach_their_main_phase(&mut engine, them());
    cast_from_hand(&mut engine, them(), grizzly_bears());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    let bears3 = on_stack(&engine, grizzly_bears()).expect("third Bears on stack");
    tap_all_mana(&mut engine, me());
    start_flashback(&mut engine, me(), denial);
    aim_at(&mut engine, me(), bears3);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, them(), grizzly_bears()).len(),
        1
    );
    assert_eq!(graveyard_len(&engine, them()), 2);
    assert!(in_graveyard(&engine, me(), fervent_denial()).is_none());
    assert!(exiled(&engine, me(), fervent_denial()));
}
