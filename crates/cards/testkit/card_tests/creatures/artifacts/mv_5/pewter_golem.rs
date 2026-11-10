//! `cards/creatures/artifacts/mv_5/pewter_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pewter_golem() -> CardIndex {
    card_index("c9d2c69e-6a4a-45b0-aff1-c19bcbe3b490")
}

/// Pewter Golem — "{1}{B}: Regenerate this creature." A bought shield saves
/// it from destruction (tapped, still on the table); without one the same
/// destruction kills it.
#[test]
fn pewter_golem_regenerates_for_one_black() {
    let p0 = PlayerId::new(0);
    for shielded in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[pewter_golem(), swamp(), forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let golem = on_battlefield(&engine, p0, pewter_golem()).expect("seated");
        assert_eq!(pt(&engine, golem), (4, 2));
        if shielded {
            tap_all_mana(&mut engine, p0);
            raise_a_shield(&mut engine, p0, golem, 0);
        }
        kill(&mut engine, golem);
        if shielded {
            assert_eq!(
                engine.state().object(golem).map(|o| o.zone),
                Some(Zone::Battlefield),
                "the shield saved it"
            );
            assert!(is_tapped(&engine, golem), "regeneration taps the permanent");
        } else {
            assert!(
                in_graveyard(&engine, p0, pewter_golem()).is_some(),
                "no shield, it dies"
            );
        }
    }
}
