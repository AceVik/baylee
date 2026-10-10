//! `cards/creatures/mv_3/desert_nomads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn desert_nomads() -> CardIndex {
    card_index("3247fca3-7458-47ee-875b-c55f2a3e2962")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn desert_land() -> CardIndex {
    card_index("195107ad-879d-4b02-a44a-a3ba70fedf88")
}

/// Attacks p1 with the Nomads and stops at p1's blocker declaration.
fn attack(engine: &mut Engine<RegistryLookup>, nomads: ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    declare_band_attack(engine, p0, p1, &[nomads]);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
}

fn try_block(engine: &mut Engine<RegistryLookup>, blocker: ObjectId, nomads: ObjectId) -> bool {
    engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, nomads)],
            },
        )
        .is_ok()
}

/// Desertwalk (CR 702.14): unblockable while the defending player controls
/// a Desert; an ordinary 2/2 body can be blocked once they do not.
#[test]
fn desert_nomads_cannot_be_blocked_while_the_defender_controls_a_desert() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (defender_desert, blockable) in [(true, false), (false, true)] {
        let lands: &[CardIndex] = if defender_desert {
            &[desert_land()]
        } else {
            &[]
        };
        let mut board = vec![gray_ogre()];
        board.extend_from_slice(lands);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[desert_nomads()])
            .battlefield(1, &board)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let nomads = on_battlefield(&engine, p0, desert_nomads()).expect("seated");
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
        attack(&mut engine, nomads);
        assert_eq!(
            try_block(&mut engine, ogre, nomads),
            blockable,
            "defender controls a Desert: {defender_desert}"
        );
    }
}

/// Only the *defending* player's Desert gives it evasion: the attacker's own
/// Desert changes nothing.
#[test]
fn desert_nomads_ignores_a_desert_its_own_controller_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[desert_nomads(), desert_land()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nomads = on_battlefield(&engine, p0, desert_nomads()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    attack(&mut engine, nomads);
    assert!(
        try_block(&mut engine, ogre, nomads),
        "the attacker's Desert is not the defender's"
    );
}

/// Damage from a Desert is prevented (the ping does nothing to the Nomads)
/// while the same ping marks a plain attacker; non-Desert damage still hits.
#[test]
fn desert_nomads_takes_no_damage_from_deserts_but_other_damage_still_hits() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[desert_nomads(), gray_ogre(), mountain()])
        .hand(0, &[shock()])
        .battlefield(1, &[desert_land(), desert_land()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nomads = on_battlefield(&engine, p0, desert_nomads()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    declare_band_attack(&mut engine, p0, p1, &[nomads, ogre]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    for target in [nomads, ogre] {
        activate(&mut engine, p1, desert_land(), 1);
        aim_at(&mut engine, p1, target);
        pass_until(&mut engine, |e| {
            stack_is_empty(e)
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
        });
    }
    let damage = |id| engine.state().object(id).map(|o| o.damage);
    assert_eq!(damage(ogre), Some(1), "the ping lands on a plain attacker");
    assert_eq!(damage(nomads), Some(0), "and is prevented for the Nomads");
}

/// Prevention is only against Deserts: a Shock still kills the 2/2.
#[test]
fn desert_nomads_still_dies_to_a_shock() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[desert_nomads(), mountain()])
        .hand(0, &[shock()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nomads = on_battlefield(&engine, p0, desert_nomads()).expect("seated");
    cast_from_hand(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![nomads],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, desert_nomads()).is_some(),
        "2 damage from a non-Desert source is not prevented"
    );
}
