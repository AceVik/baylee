//! Mesmeric Orb mills the controller of a permanent that untaps.

use super::*;
use baylee_core::generated::index;

fn mesmeric_orb() -> CardIndex {
    card_index("03efb4f3-b8e2-4441-824f-886dc40712c4")
}

#[test]
fn mesmeric_orb_mills_when_voltaic_key_untaps_an_artifact() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mesmeric_orb(), voltaic_key(), quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ring = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    tap_all_mana_but(&mut e, p0, Some(voltaic_key()));
    assert!(is_tapped(&e, ring));
    let before = library_size(&e, p0);
    activate(&mut e, p0, voltaic_key(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ring],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, ring));
    assert_eq!(library_size(&e, p0), before - 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 1);
}

/// "That permanent's controller mills": the opponent's lands untapping in
/// their own untap step mill the opponent, one card each, and not the Orb's
/// controller.
#[test]
fn mesmeric_orb_mills_the_untapped_permanent_s_controller_in_their_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mesmeric_orb()])
        .battlefield(1, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let lands = all_on_battlefield(&e, p1, mountain());
    let state = e.dev_state_mut(p0).unwrap();
    for &land in &lands {
        state.set_tapped(land, true);
    }
    e.refresh_offer();
    assert!(lands.iter().all(|&land| is_tapped(&e, land)));
    reach_their_main_phase(&mut e, p1);
    assert!(lands.iter().all(|&land| !is_tapped(&e, land)));
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p1)).len(), 2);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 0);
}

#[test]
fn mesmeric_orb_and_wake_thrasher_read_control_when_the_creature_untaps() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    for (spell, controller_on_untap) in [
        (index::RAY_OF_COMMAND, them),
        (index::CAUGHT_RED_HANDED, me),
    ] {
        let mut engine = Duel::new(SEED, forest())
            .hand(0, &[spell])
            .battlefield(
                0,
                &[
                    mesmeric_orb(),
                    index::WAKE_THRASHER,
                    island(),
                    island(),
                    island(),
                    island(),
                    mountain(),
                ],
            )
            .battlefield(1, &[index::WAKE_THRASHER])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, me);

        let mine = on_battlefield(&engine, me, index::WAKE_THRASHER).unwrap();
        let target = on_battlefield(&engine, them, index::WAKE_THRASHER).unwrap();
        assert!(engine.dev_state_mut(them).unwrap().set_tapped(target, true));
        engine.refresh_offer();
        let library_before = [library_size(&engine, me), library_size(&engine, them)];

        cast_from_hand(&mut engine, me, spell);
        aim_at(&mut engine, me, target);
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(engine.state().object(target).unwrap().controller, me);
        for (seat, before) in [me, them].into_iter().zip(library_before) {
            assert_eq!(
                library_size(&engine, seat),
                before - usize::from(seat == controller_on_untap),
                "Mesmeric Orb mills the player who controlled the creature when it untapped"
            );
        }
        let mine_power = if controller_on_untap == me { 2 } else { 1 };
        assert_eq!(pt(&engine, mine), (mine_power, mine_power));
        assert_eq!(pt(&engine, target), (2, 2));
        assert!(engine.journal().entries().iter().any(|entry| {
            matches!(entry.event, GameEvent::AbilityTriggered { source, controller, .. }
                if source == target && controller == controller_on_untap)
        }));
    }
}
