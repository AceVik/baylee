//! `cards/creatures/mv_4/sawtooth_ogre.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AIR_ELEMENTAL, SAWTOOTH_OGRE};

/// `Sawtooth Ogre` ({2}{R}{R}, Creature — Ogre 3/3) deals 1 damage to each
/// creature it blocks or becomes blocked by at end of combat.
#[test]
fn sawtooth_ogre_deals_one_damage_to_blocker_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[SAWTOOTH_OGRE])
        .battlefield(1, &[AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, SAWTOOTH_OGRE).unwrap();
    let elemental = on_battlefield(&engine, p1, AIR_ELEMENTAL).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elemental, ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });

    assert!(in_graveyard(&engine, p0, SAWTOOTH_OGRE).is_some());
    assert!(in_graveyard(&engine, p1, AIR_ELEMENTAL).is_some());
}
