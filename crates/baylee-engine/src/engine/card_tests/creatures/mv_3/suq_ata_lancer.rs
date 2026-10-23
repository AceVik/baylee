//! `cards/creatures/mv_3/suq_ata_lancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MOUNTAIN};

fn suq_ata_lancer() -> CardIndex {
    card_index("f348b326-8a73-4df9-8faf-e271a36ce373")
}

/// Casts P0's Lancer, attacks with it the turn it enters (haste) and has
/// `blocker` block it. Returns the Lancer once the blocks are declared.
fn cast_attack_and_block(engine: &mut Engine<RegistryLookup>, blocker: ObjectId) -> ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    cast_from_hand(engine, p0, suq_ata_lancer());
    pass_until(engine, stack_is_empty);
    let lancer = on_battlefield(engine, p0, suq_ata_lancer()).unwrap();

    // Haste lets Suq'Ata Lancer attack the turn it enters
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lancer, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, lancer)],
            },
        )
        .unwrap();
    lancer
}

/// `Suq'Ata Lancer` ({2}{R}, 2/2 Creature — Human Knight,
/// `Coverage::Implemented`) has haste and flanking, shrinking a non-flanking
/// blocker by -1/-1 until end of turn.
#[test]
fn suq_ata_lancer_has_haste_and_flanking_shrinks_non_flanking_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN, MOUNTAIN])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[suq_ata_lancer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_attack_and_block(&mut engine, blocker);
    pass_until(&mut engine, stack_is_empty);

    // Exactly one -1/-1: a 2/2 Bears becomes 1/1.
    assert_eq!(pt(&engine, blocker), (1, 1));
}

/// "A creature without flanking": a blocker that has flanking itself is not
/// shrunk.
#[test]
fn suq_ata_lancer_does_not_shrink_a_blocker_with_flanking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN, MOUNTAIN])
        .battlefield(1, &[suq_ata_lancer()])
        .hand(0, &[suq_ata_lancer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let blocker = on_battlefield(&engine, p1, suq_ata_lancer()).unwrap();
    cast_attack_and_block(&mut engine, blocker);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, blocker),
        (2, 2),
        "a flanking blocker keeps its size"
    );
}
