//! `cards/artifacts/mv_2/powder_keg.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn powder_keg() -> CardIndex {
    card_index("ca1144c2-49a3-49b4-9085-3094141769ea")
}

/// Powder Keg (ca1144c2-49a3-49b4-9085-3094141769ea) — {2} Artifact:
/// "At the beginning of your upkeep, you may put a fuse counter on this artifact."
#[test]
fn powder_keg_upkeep_may_add_or_decline_a_fuse_counter() {
    use baylee_cards_dsl::counters;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[powder_keg()])
        .start();
    keep_mulligans(&mut engine);

    let keg = on_battlefield(&engine, p0, powder_keg()).unwrap();

    // Upkeep 1: decline the optional fuse counter.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert_eq!(counters_on(&engine, keg, counters::FUSE), 0);

    // Upkeep 2: accept the optional fuse counter.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::MayDo,
                    ..
                }
            )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert_eq!(counters_on(&engine, keg, counters::FUSE), 1);
}

/// "{T}, Sacrifice this artifact: Destroy each artifact and creature with mana
/// value equal to the number of fuse counters on this artifact."
#[test]
fn powder_keg_sweeps_artifacts_and_creatures_matching_lki_fuse_counters() {
    use baylee_cards_dsl::counters;
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                powder_keg(),
                index::MOX_EMERALD,
                llanowar_elves(),
                index::GRIZZLY_BEARS,
                forest(),
            ],
        )
        .battlefield(
            1,
            &[
                index::MOX_SAPPHIRE,
                llanowar_elves(),
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    let keg = on_battlefield(&engine, p0, powder_keg()).unwrap();
    let my_mox = on_battlefield(&engine, p0, index::MOX_EMERALD).unwrap();
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let my_bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let my_land = on_battlefield(&engine, p0, forest()).unwrap();
    let their_mox = on_battlefield(&engine, p1, index::MOX_SAPPHIRE).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let their_bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let their_giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    assert_eq!(counters_on(&engine, keg, counters::FUSE), 1);

    // Cannot activate when already tapped.
    engine.dev_state_mut(p0).unwrap().set_tapped(keg, true);
    engine.refresh_offer();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: keg,
                    ability_index: 1,
                },
            )
            .is_err(),
        "tapped Powder Keg cannot activate {{T}}, Sacrifice this artifact"
    );
    engine.dev_state_mut(p0).unwrap().set_tapped(keg, false);
    engine.refresh_offer();

    activate(&mut engine, p0, powder_keg(), 1);
    assert_eq!(engine.state().object(keg).unwrap().zone, Zone::Graveyard);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(my_elf).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(their_elf).unwrap().zone,
        Zone::Graveyard
    );
    for spared in [
        my_mox,
        their_mox,
        my_bears,
        their_bears,
        their_giant,
        my_land,
    ] {
        assert_eq!(
            engine.state().object(spared).unwrap().zone,
            Zone::Battlefield
        );
    }
}
