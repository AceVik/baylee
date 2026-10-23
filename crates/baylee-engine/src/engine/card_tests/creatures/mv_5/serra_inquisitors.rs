//! Serra Inquisitors — `{4}{W}` 3/3 Creature — Human Cleric (Homelands)
//!
//! Whenever this creature blocks or becomes blocked by one or more black creatures, this creature
//! gets +2/+0 until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_inquisitors() -> CardIndex {
    card_index("3c6a32e8-0c2a-45cb-8b79-c7f022fe5129")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

/// A black 2/2.
fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

/// A red 3/3.
fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn permanents_named(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// P0 attacks with the Inquisitors into P1's board, and every P1 creature named `blockers` blocks
/// it. Returns the engine once the declare-blockers step has settled (triggers resolved, no damage
/// dealt yet) and the Inquisitors.
fn inquisitors_attacks(
    p1_board: &[CardIndex],
    blockers: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[serra_inquisitors()])
        .battlefield(1, p1_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let inquisitors = on_battlefield(&engine, p0, serra_inquisitors()).unwrap();
    let blocking: Vec<_> = permanents_named(&engine, p1, blockers)
        .into_iter()
        .map(|b| (b, inquisitors))
        .collect();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(inquisitors, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: blocking })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, inquisitors)
}

/// P1 attacks with `attacker` and the Inquisitors block it. Returns the engine once the
/// declare-blockers step has settled, and the Inquisitors.
fn inquisitors_blocks(attacker: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[serra_inquisitors()])
        .battlefield(1, &[attacker])
        .start();
    keep_mulligans(&mut engine);

    let inquisitors = on_battlefield(&engine, p0, serra_inquisitors()).unwrap();
    let attacking = on_battlefield(&engine, p1, attacker).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacking, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(inquisitors, attacking)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, inquisitors)
}

#[test]
fn serra_inquisitors_gets_plus_two_plus_zero_when_blocked_by_black_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[serra_inquisitors()])
        .battlefield(1, &[sengir_vampire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let inquisitors = on_battlefield(&engine, p0, serra_inquisitors()).unwrap();
    let vampire = on_battlefield(&engine, p1, sengir_vampire()).unwrap();
    assert_eq!(pt(&engine, inquisitors), (3, 3));

    // Attack with 3/3 Serra Inquisitors; p1 blocks with 4/4 black Sengir Vampire.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(inquisitors, Defender::Player(p1))],
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
                blockers: vec![(vampire, inquisitors)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Serra Inquisitors became 5/3 and traded with the 4/4 Sengir Vampire!
    assert_eq!(
        engine.state().object(vampire).unwrap().zone,
        Zone::Graveyard,
        "5/3 Serra Inquisitors killed the 4/4 black Sengir Vampire"
    );
}

#[test]
fn serra_inquisitors_gets_plus_two_until_end_of_turn_when_blocked_by_a_black_creature() {
    let p0 = PlayerId::new(0);
    // A black 2/2 blocks the 3/3: it becomes 5/3 and survives the 2 damage.
    let (mut engine, inquisitors) = inquisitors_attacks(&[scathe_zombies()], scathe_zombies());
    assert_eq!(pt(&engine, inquisitors), (5, 3));

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(inquisitors).unwrap().zone,
        Zone::Battlefield
    );
    assert!(on_battlefield(&engine, PlayerId::new(1), scathe_zombies()).is_none());

    // Until end of turn: on its controller's next turn it is a 3/3 again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, inquisitors), (3, 3));
}

#[test]
fn serra_inquisitors_gets_plus_two_when_it_blocks_a_black_creature() {
    let (engine, inquisitors) = inquisitors_blocks(scathe_zombies());
    assert_eq!(pt(&engine, inquisitors), (5, 3));
}

#[test]
fn serra_inquisitors_gets_nothing_against_a_nonblack_creature_either_way() {
    // Blocked by a red Hill Giant.
    let (engine, inquisitors) = inquisitors_attacks(&[hill_giant()], hill_giant());
    assert_eq!(pt(&engine, inquisitors), (3, 3));

    // Blocking a red Hill Giant.
    let (engine, inquisitors) = inquisitors_blocks(hill_giant());
    assert_eq!(pt(&engine, inquisitors), (3, 3));
}

/// "One or more black creatures": two black blockers are one trigger, +2/+0 once.
#[test]
fn serra_inquisitors_gets_plus_two_once_when_blocked_by_two_black_creatures() {
    let (engine, inquisitors) =
        inquisitors_attacks(&[scathe_zombies(), scathe_zombies()], scathe_zombies());
    assert_eq!(pt(&engine, inquisitors), (5, 3));
}
