//! `cards/enchantments/mv_1/tidal_flats.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flats_card() -> CardIndex {
    card_index("bf39d0f7-f689-45fd-b37b-12d1618fd6e7")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}
fn soldiers_card() -> CardIndex {
    card_index("0e3ac78c-8a98-4786-b86c-ecfe61e13c83")
}
fn air_elemental_card() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// P0 attacks with `attackers`, P1 (holding Tidal Flats and Vodalian
/// Soldiers) blocks the Bears with the Soldiers when `block` is set, and the
/// game stops with P1 holding priority in the declare blockers step.
fn combat(
    attackers: &[CardIndex],
    block: bool,
) -> (Engine<RegistryLookup>, Vec<ObjectId>, ObjectId, ObjectId) {
    let mut e = table(SEED, &[], attackers, &[flats_card(), soldiers_card()]);
    let ids: Vec<ObjectId> = attackers
        .iter()
        .map(|c| on_battlefield(&e, P0, *c).expect("an attacker is out"))
        .collect();
    let soldiers = on_battlefield(&e, P1, soldiers_card()).expect("Vodalian Soldiers");
    let flats = on_battlefield(&e, P1, flats_card()).expect("Tidal Flats");

    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: ids.iter().map(|id| (*id, Defender::Player(P1))).collect(),
        },
    )
    .expect("P0 attacks");
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    let blockers = if block {
        vec![(soldiers, ids[0])]
    } else {
        vec![]
    };
    e.apply(P1, PlayerAction::DeclareBlockers { blockers })
        .expect("P1 declares blockers");
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    (e, ids, soldiers, flats)
}

/// Activates Tidal Flats and runs until either the attacker's controller is
/// asked whether to pay or the stack is empty.
fn activate_flats(e: &mut Engine<RegistryLookup>, flats: ObjectId) {
    fill_pool(e);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: flats,
            ability_index: 0,
        },
    )
    .expect("Tidal Flats activates for {U}{U}");
    pass_until(e, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
}

fn has_first_strike(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    keywords(e, id).contains(KeywordSet::FIRST_STRIKE)
}

/// Tidal Flats (bf39d0f7-f689-45fd-b37b-12d1618fd6e7) — "{U}{U}: For each
/// attacking creature without flying, its controller may pay {1}. If that
/// player doesn't, creatures you control blocking that creature gain first
/// strike until end of turn." The attacker's controller declines.
#[test]
fn tidal_flats_gives_the_blocker_first_strike_when_the_attacker_does_not_pay() {
    let (mut e, ids, soldiers, flats) = combat(&[bears_card()], true);
    assert!(!has_first_strike(&e, soldiers), "no first strike to start");
    let _ = ids;

    activate_flats(&mut e, flats);
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("the attacker's controller is asked, got {:?}", e.pending())
    };
    assert_eq!(player, P0, "the attacking creature's controller decides");
    e.apply(P0, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, stack_is_empty);

    assert!(
        has_first_strike(&e, soldiers),
        "the blocker gained first strike"
    );
}

/// Tidal Flats — the other side of "if that player doesn't": paying {1}
/// spares the attacker, and the blocker gets nothing.
#[test]
fn tidal_flats_gives_nothing_when_the_attacker_pays_one() {
    let (mut e, _ids, soldiers, flats) = combat(&[bears_card()], true);

    activate_flats(&mut e, flats);
    let before = e.state.players[0].mana_pool.total();
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("the attacker's controller is asked, got {:?}", e.pending())
    };
    assert_eq!(player, P0);
    e.apply(P0, PlayerAction::YesNo(true)).expect("pay");
    pass_until(&mut e, stack_is_empty);

    assert!(
        !has_first_strike(&e, soldiers),
        "the attacker paid, so no first strike"
    );
    assert_eq!(
        before - e.state.players[0].mana_pool.total(),
        1,
        "exactly {{1}} was paid"
    );
}

/// Tidal Flats — "each attacking creature without flying": a flier is not
/// asked about, and its blockers get nothing; the same activation still
/// asks once for the ground attacker.
#[test]
fn tidal_flats_ignores_attacking_creatures_with_flying() {
    let (mut e, ids, soldiers, flats) = combat(&[air_elemental_card()], false);
    let _ = ids;
    activate_flats(&mut e, flats);
    assert!(
        !matches!(e.pending(), Pending::YesNo { .. }),
        "nobody is asked about a flying attacker, got {:?}",
        e.pending()
    );
    assert!(stack_is_empty(&e));
    assert!(!has_first_strike(&e, soldiers));

    let (mut e, ids, soldiers, flats) = combat(&[bears_card(), air_elemental_card()], true);
    let _ = ids;
    activate_flats(&mut e, flats);
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("the ground attacker is asked about, got {:?}", e.pending())
    };
    assert_eq!(player, P0);
    e.apply(P0, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    assert!(
        !matches!(e.pending(), Pending::YesNo { .. }),
        "only the one non-flying attacker was asked about"
    );
    assert!(has_first_strike(&e, soldiers));
}
