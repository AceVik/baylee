//! `cards/artifacts/mv_6/runesword.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn runesword() -> CardIndex {
    card_index("964c2003-cef0-4ac7-9f66-4e20893c8e50")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn draconian_cylix() -> CardIndex {
    card_index("0880b36a-6141-4955-b532-cf88daca0869")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

fn zone_of(engine: &Engine<RegistryLookup>, id: ObjectId) -> Zone {
    engine.state().object(id).expect("the object exists").zone
}

/// p0's attackers go at p1; the engine is left with p0 holding priority in
/// the declare-attackers step.
#[track_caller]
fn declare(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attackers were on the offer");
}

/// Pays {3} with the Forests, aims the Runesword at `attacker` and resolves it.
#[track_caller]
fn runesword_at(engine: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    let p0 = PlayerId::new(0);
    tap_all_mana(engine, p0);
    activate(engine, p0, runesword(), 0);
    aim_at(engine, p0, attacker);
    pass_until(engine, stack_is_empty);
}

/// p0: Runesword, Bears, six Forests. p1: `theirs`. p0's first main phase.
fn a_sword_board(theirs: &[CardIndex], p1_hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut mine = vec![runesword(), grizzly_bears()];
    mine.extend(std::iter::repeat_n(forest(), 6));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, theirs)
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Runesword (964c2003-cef0-4ac7-9f66-4e20893c8e50) — "{3}, {T}: Target
/// attacking creature gets +2/+0 until end of turn. When that creature leaves
/// the battlefield this turn, sacrifice this artifact. If the creature deals
/// damage to a creature this turn, the creature dealt damage can't be
/// regenerated this turn. If a creature dealt damage by the targeted creature
/// would die this turn, exile that creature instead."
///
/// Only an *attacking* creature is a target: in the main phase, with {3}
/// floating, the ability is not on offer. Once the Bears attack, the Sword
/// makes them 4/2 and they deal 4 unblocked; the Bears did not leave, so the
/// Sword stays.
#[test]
fn runesword_pumps_an_attacker_and_stays_while_it_lives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_sword_board(&[], &[]);
    let sword = on_battlefield(&engine, p0, runesword()).expect("the Sword");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    let forests = all_on_battlefield(&engine, p0, forest());
    let wasted = forests[..3].to_vec();
    tap_mana_where(&mut engine, p0, |id| wasted.contains(&id));
    assert!(
        !priority_offer(&engine).abilities.contains(&(sword, 0)),
        "nothing is attacking yet: no target, no activation"
    );

    declare(&mut engine, &[bears]);
    let rest = forests[3..].to_vec();
    assert_eq!(tap_mana_where(&mut engine, p0, |id| rest.contains(&id)), 3);
    activate(&mut engine, p0, runesword(), 0);
    aim_at(&mut engine, p0, bears);
    assert!(is_tapped(&engine, sword), "{{T}} was paid");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 2), "+2/+0");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(life(&engine, p1), 16, "four unblocked");
    assert!(
        on_battlefield(&engine, p0, runesword()).is_some(),
        "the Bears never left the battlefield"
    );
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}

/// The Giant blocks the 4/2 Bears: it takes 4, would die, and is exiled
/// instead; the Bears take 3, die, and the Sword is sacrificed.
#[test]
fn runesword_exiles_what_the_pumped_creature_kills_and_is_sacrificed_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_sword_board(&[hill_giant()], &[]);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    declare(&mut engine, &[bears]);
    runesword_at(&mut engine, bears);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });

    assert_eq!(zone_of(&engine, giant), Zone::Exile, "exiled, not dead");
    assert!(
        in_graveyard(&engine, p1, hill_giant()).is_none(),
        "nothing reached their graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "the Bears died"
    );
    assert!(
        on_battlefield(&engine, p0, runesword()).is_none(),
        "the Bears left: the Sword was sacrificed"
    );
    assert!(in_graveyard(&engine, p0, runesword()).is_some());
}

/// "Can't be regenerated": the Giant has a Cylix shield standing, raised
/// after the blocks. Ordinary lethal damage would be regenerated and the Giant
/// would live on; the Sword's creature dealt the damage, so the shield is
/// void, the Giant dies and is exiled.
#[test]
fn runesword_damage_cannot_be_regenerated_away() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_sword_board(
        &[hill_giant(), draconian_cylix(), forest(), forest()],
        &[island()],
    );
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    declare(&mut engine, &[bears]);
    runesword_at(&mut engine, bears);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .unwrap();

    // p1 answers with the Cylix before damage.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, draconian_cylix(), 0);
    aim_at(&mut engine, p1, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(giant).unwrap().regeneration_shields,
        1,
        "the shield stands before damage"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(
        zone_of(&engine, giant),
        Zone::Exile,
        "the shield did not save it, and the Sword exiled it"
    );
}
