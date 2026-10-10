//! `cards/creatures/mv_1/camel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn camel() -> CardIndex {
    card_index("d33b3591-c01f-4ac4-8626-7cbfdabaf90d")
}

fn desert_land() -> CardIndex {
    card_index("195107ad-879d-4b02-a44a-a3ba70fedf88")
}

/// Camel (0/1, banding) attacks with a Minotaur and a Craw Wurm; the
/// Minotaur is banded with it or not. Every attacker is then pinged by a
/// Desert at end of combat. Returns (camel, minotaur, wurm) and the engine.
fn desert_after_attack(banded: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[camel(), hurloon_minotaur(), craw_wurm()])
        .battlefield(1, &[desert_land(), desert_land(), desert_land()])
        .life(1, 40)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let camel_id = on_battlefield(&engine, p0, camel()).expect("seated");
    let minotaur = on_battlefield(&engine, p0, hurloon_minotaur()).expect("seated");
    let wurm = on_battlefield(&engine, p0, craw_wurm()).expect("seated");
    declare_band_attack(&mut engine, p0, p1, &[camel_id, minotaur, wurm]);
    let band = if banded { vec![minotaur] } else { vec![] };
    answer_band_and_reach_blockers(&mut engine, p0, band);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    for target in [camel_id, minotaur, wurm] {
        activate(&mut engine, p1, desert_land(), 1);
        aim_at(&mut engine, p1, target);
        pass_until(&mut engine, |e| {
            stack_is_empty(e)
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
        });
    }
    (engine, camel_id, minotaur, wurm)
}

fn damage(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<u32> {
    engine.state().object(id).map(|o| u32::from(o.damage))
}

/// While attacking, Camel and the creature banded with it take nothing from
/// Deserts; the attacker outside the band is hurt as usual.
#[test]
fn camel_shields_itself_and_its_band_from_deserts_but_not_other_attackers() {
    let (engine, camel_id, minotaur, wurm) = desert_after_attack(true);
    assert!(
        engine.state().object(camel_id).is_some(),
        "a 0/1 pinged by a Desert would have died"
    );
    assert_eq!(damage(&engine, camel_id), Some(0), "Camel itself");
    assert_eq!(damage(&engine, minotaur), Some(0), "banded with Camel");
    assert_eq!(damage(&engine, wurm), Some(1), "not in the band");
}

/// Without the band, the Minotaur gets no protection from Camel.
#[test]
fn camel_does_not_shield_an_attacker_it_is_not_banded_with() {
    let (engine, camel_id, minotaur, wurm) = desert_after_attack(false);
    assert_eq!(damage(&engine, camel_id), Some(0), "Camel still shielded");
    assert_eq!(damage(&engine, minotaur), Some(1), "no band, no shield");
    assert_eq!(damage(&engine, wurm), Some(1));
}
