//! `cards/instants/mv_4/sewers_of_estark.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sewers_of_estark() -> CardIndex {
    card_index("2482f98a-eadf-450d-9e73-fc441b572862")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Passes priority until `seat` holds it.
#[track_caller]
fn priority_for(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == seat => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected priority on the way to {seat:?}, got {other:?}"),
        }
    }
    panic!("{seat:?} never got priority");
}

/// p0 holds a Sewers of Estark and four Swamps, with Bears out; p1 has a
/// Hill Giant. Both are on the battlefield from the start of the game.
fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[sewers_of_estark()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    engine
}

/// Sewers of Estark (2482f98a-eadf-450d-9e73-fc441b572862) — "Choose target
/// creature. If it's attacking, it can't be blocked this turn. If it's
/// blocking, prevent all combat damage that would be dealt this combat by it
/// and each creature it's blocking."
///
/// First sentence: cast on my attacking Bears, the Giant that would
/// otherwise eat them has nothing to block, and the damage reaches the
/// player.
#[test]
fn sewers_of_estark_makes_an_attacking_creature_unblockable() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .expect("the Bears attack");

    cast_from_hand(&mut engine, p0, sewers_of_estark());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, bears).contains(KeywordSet::UNBLOCKABLE),
        "the attacking Bears can't be blocked"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || matches!(e.state().turn.phase, Phase::SecondMain)
    });
    match engine.pending().clone() {
        Pending::ChooseBlockers { blockers, .. } => {
            assert!(
                blockers.iter().all(|o| !o.attackers.contains(&bears)),
                "nothing may block the Bears: {blockers:?}"
            );
            engine
                .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
                .expect("no blocks");
        }
        // The engine may also skip a block declaration nobody can make.
        Pending::Priority { .. } => {}
        other => panic!("unexpected at the end of the walk: {other:?}"),
    }
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 18, "the Bears connected");
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "and were never blocked"
    );
}

/// The counter-board: without the Sewers, the Giant is offered the block.
#[test]
fn without_sewers_the_giant_may_block_the_attacking_bears() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");

    let blocks = attack_and_collect_blocks(&mut engine, bears, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == giant && o.attackers.contains(&bears)),
        "the Giant may block the Bears: {blocks:?}"
    );
}

/// The Sewers choose any creature, but "if it's attacking" is read when it
/// resolves: cast on Bears that are not attacking, it changes nothing, and
/// they can be blocked when they do attack.
#[test]
fn sewers_of_estark_on_a_creature_that_is_not_attacking_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");

    cast_from_hand(&mut engine, p0, sewers_of_estark());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::UNBLOCKABLE),
        "not attacking, so not unblockable"
    );

    let blocks = attack_and_collect_blocks(&mut engine, bears, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == giant && o.attackers.contains(&bears)),
        "the Giant may still block the Bears: {blocks:?}"
    );
}

/// Second sentence: cast on my blocking Bears, no combat damage is dealt by
/// them or by the Giant they are blocking, so both survive a fight the Bears
/// would lose.
#[test]
fn sewers_of_estark_prevents_the_damage_of_a_blocker_and_what_it_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");
    // The Sewers are cast in p1's turn, so p0 keeps its Swamps untapped.
    reach_their_main_phase(&mut engine, p1);

    let blocks = attack_and_collect_blocks(&mut engine, giant, p0);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == bears && o.attackers.contains(&giant)),
        "the Bears may block the Giant: {blocks:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant)],
            },
        )
        .expect("the Bears block the Giant");
    priority_for(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sewers_of_estark());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    let bears_object = engine
        .state()
        .object(bears)
        .expect("the Bears are still there");
    assert_eq!(
        bears_object.zone,
        Zone::Battlefield,
        "the Giant's damage was prevented"
    );
    assert_eq!(bears_object.damage, 0);
    let giant_object = engine.state().object(giant).expect("the Giant is there");
    assert_eq!(giant_object.damage, 0, "and the Bears' damage too");
    assert_eq!(engine.state().players[0].life, 20, "the Giant was blocked");
}
