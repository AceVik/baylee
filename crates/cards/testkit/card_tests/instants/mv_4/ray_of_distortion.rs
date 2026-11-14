//! `cards/2001/src/ody/ray_of_distortion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ray_of_distortion() -> CardIndex {
    card_index("1c838441-4713-49dd-97e6-4fb7155c36c8")
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

/// Destroys target artifact or enchantment (never creature or land), refuses
/// Flashback with 5 mana, and destroys a second permanent via Flashback {4}{W}{W}
/// before going to exile.
#[test]
fn ray_of_distortion_destroys_artifact_and_enchantment_with_flashback() {
    // 9 Plains: 4 spent on {3}{W} from hand leaves 5 untapped for the 5-mana
    // Flashback {4}{W}{W} boundary; all 9 untap on the following P0 main phase.
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 9])
        .battlefield(
            1,
            &[
                quiet_artifact(),
                their_enchantment(),
                grizzly_bears(),
                forest(),
            ],
        )
        .hand(0, &[ray_of_distortion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ring = on_battlefield(&engine, P1, quiet_artifact()).unwrap();
    let chant = on_battlefield(&engine, P1, their_enchantment()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let land = on_battlefield(&engine, P1, forest()).unwrap();

    cast_from_hand(&mut engine, P0, ray_of_distortion());
    let options = aim_at(&mut engine, P0, ring);
    assert!(options.contains(&ring) && options.contains(&chant));
    assert!(!options.contains(&bears) && !options.contains(&land));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(ring).unwrap().zone, Zone::Graveyard);
    let ray = in_graveyard(&engine, P0, ray_of_distortion()).expect("Ray in graveyard");

    // 5 floating mana, 0 untapped Plains -> Flashback {4}{W}{W} (6) refuses.
    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(!legal.castable.contains(&ray));
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: ray })
            .is_err()
    );

    // Next P0 main with 9 Plains untapping: Flashback {4}{W}{W} destroys chant & exiles Ray.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    start_flashback(&mut engine, P0, ray);
    let fb_options = aim_at(&mut engine, P0, chant);
    assert_eq!(fb_options, vec![chant]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(chant).unwrap().zone, Zone::Graveyard);
    assert!(in_graveyard(&engine, P0, ray_of_distortion()).is_none());
    assert_eq!(engine.state().object(ray).unwrap().zone, Zone::Exile);
}
