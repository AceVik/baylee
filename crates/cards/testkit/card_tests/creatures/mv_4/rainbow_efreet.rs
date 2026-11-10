//! `cards/creatures/mv_4/rainbow_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rainbow_efreet() -> CardIndex {
    card_index("7c158405-d3ef-4c10-a504-7b061ef71a74")
}

/// `Rainbow Efreet` ({3}{U}, 3/1 Creature — Efreet, `Coverage::Implemented`)
/// has flying and `{U}{U}: This creature phases out.`
#[test]
fn rainbow_efreet_has_flying_and_phases_out_for_two_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[rainbow_efreet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rainbow_efreet());
    pass_until(&mut engine, stack_is_empty);
    let efreet = on_battlefield(&engine, p0, rainbow_efreet()).unwrap();
    assert!(keywords(&engine, efreet).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, rainbow_efreet(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}

/// "It phases in before you untap during your next untap step": phased out on
/// its controller's turn, it is still out through the opponent's turn and is
/// back on the battlefield on its controller's next turn.
#[test]
fn rainbow_efreet_phases_in_at_its_controllers_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[rainbow_efreet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rainbow_efreet());
    pass_until(&mut engine, stack_is_empty);
    let efreet = on_battlefield(&engine, p0, rainbow_efreet()).unwrap();
    activate(&mut engine, p0, rainbow_efreet(), 0);
    pass_until(&mut engine, stack_is_empty);
    let phased_out = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    };
    assert!(phased_out(&engine));

    reach_their_main_phase(&mut engine, p1);
    assert!(phased_out(&engine), "still out through the opponent's turn");
    reach_their_main_phase(&mut engine, p0);
    assert!(!phased_out(&engine), "back on its controller's next turn");
}
