//! `cards/creatures/mv_6/giant_shark.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).expect("the spell is in hand");
    e.apply(p, PlayerAction::CastSpell { card })
        .expect("the spell is cast");
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects,
            players: vec![],
        },
    )
    .expect("the targets are on the menu");
}

fn giant_shark() -> CardIndex {
    card_index("44a10a63-be9c-4f1d-aad6-b5337112bda5")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The attackers P0's declare-attackers question offers.
fn offered_attackers(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(e, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    attackers
}

/// One combat between the Shark (P0, with an Island of its own) and P1's Earth
/// Elemental (4/5, vanilla), stopped right after the blockers are declared.
/// With `damaged`, P0 first burns the Elemental with Lightning Bolt (3 damage,
/// which it survives), so one of the two creatures in the fight has been dealt
/// damage this turn.
fn shark_in_combat(shark_attacks: bool, damaged: bool) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = prepared(
        &[lightning_bolt()],
        &[giant_shark(), island()],
        &[earth_elemental(), island()],
    );
    let shark = on_battlefield(&e, P0, giant_shark()).expect("the Shark is out");
    let elemental = on_battlefield(&e, P1, earth_elemental()).expect("Earth Elemental is out");

    if shark_attacks {
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shark, Defender::Player(P1))],
            },
        )
        .expect("the Shark attacks: P1 controls an Island");
    } else {
        pass_until(&mut e, |e| {
            e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
        });
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        e.apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elemental, Defender::Player(P0))],
            },
        )
        .expect("the Elemental attacks");
    }

    if damaged {
        cast(&mut e, P0, lightning_bolt());
        targets(&mut e, vec![elemental]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(elemental).unwrap().damage, 3);
    }

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let (defender, pair) = if shark_attacks {
        (P1, (elemental, shark))
    } else {
        (P0, (shark, elemental))
    };
    e.apply(
        defender,
        PlayerAction::DeclareBlockers {
            blockers: vec![pair],
        },
    )
    .expect("the block is legal");
    (e, shark)
}

/// Giant Shark (44a10a63-be9c-4f1d-aad6-b5337112bda5) — "This creature can't
/// attack unless defending player controls an Island."
#[test]
fn giant_shark_attacks_only_a_player_who_controls_an_island() {
    // No Island on the other side: not offered.
    let mut e = prepared(
        &[],
        &[giant_shark(), island(), grizzly_bears()],
        &[mountain()],
    );
    let shark = on_battlefield(&e, P0, giant_shark()).expect("the Shark is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let attackers = offered_attackers(&mut e);
    assert!(attackers.contains(&bears));
    assert!(!attackers.contains(&shark), "no Island across the table");
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shark, Defender::Player(P1))],
            },
        )
        .is_err(),
        "the declaration is refused"
    );

    // An Island across the table: offered, and the attack goes through.
    let mut e = prepared(
        &[],
        &[giant_shark(), island(), grizzly_bears()],
        &[island()],
    );
    let shark = on_battlefield(&e, P0, giant_shark()).expect("the Shark is out");
    let attackers = offered_attackers(&mut e);
    assert!(attackers.contains(&shark), "an Island across the table");
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(shark, Defender::Player(P1))],
        },
    )
    .expect("the Shark attacks");
}

/// Giant Shark — "When you control no Islands, sacrifice this creature."
#[test]
fn giant_shark_is_sacrificed_when_its_controller_has_no_islands() {
    // Without any Island from the start.
    let mut e = prepared(&[], &[giant_shark()], &[island()]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, giant_shark()).is_some(),
        "no Island of its own: sacrificed (the opponent's Island does not count)"
    );

    // With two: losing one is not enough, losing the second is.
    let mut e = prepared(&[], &[giant_shark(), island(), island()], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, giant_shark()).is_some(),
        "two Islands"
    );
    let islands = lands_of(&e, P0);
    assert_eq!(islands.len(), 2);
    kill(&mut e, islands[0]);
    assert!(
        on_battlefield(&e, P0, giant_shark()).is_some(),
        "one Island is still an Island"
    );
    kill(&mut e, islands[1]);
    assert!(
        on_battlefield(&e, P0, giant_shark()).is_none(),
        "the last Island is gone"
    );
    assert!(in_graveyard(&e, P0, giant_shark()).is_some());
}

/// Giant Shark — "Whenever this creature blocks or becomes blocked by a
/// creature that has been dealt damage this turn, this creature gets +2/+0 and
/// gains trample until end of turn." (becomes blocked, damaged blocker)
#[test]
fn giant_shark_blocked_by_a_damaged_creature_gets_plus_two_and_trample() {
    let (mut e, shark) = shark_in_combat(true, true);
    pass_until(&mut e, |e| pt(e, shark) == (6, 4));
    assert!(keywords(&e, shark).contains(KeywordSet::TRAMPLE));
}

/// Giant Shark — (blocks, damaged attacker)
#[test]
fn giant_shark_blocking_a_damaged_creature_gets_plus_two_and_trample() {
    let (mut e, shark) = shark_in_combat(false, true);
    pass_until(&mut e, |e| pt(e, shark) == (6, 4));
    assert!(keywords(&e, shark).contains(KeywordSet::TRAMPLE));
}

/// Giant Shark — nothing happens against a creature that has not been damaged.
#[test]
fn giant_shark_against_an_undamaged_creature_stays_a_four_four() {
    for shark_attacks in [true, false] {
        let (e, shark) = shark_in_combat(shark_attacks, false);
        assert!(stack_is_empty(&e), "nothing triggered");
        assert_eq!(pt(&e, shark), (4, 4));
        assert!(!keywords(&e, shark).contains(KeywordSet::TRAMPLE));
    }
}
