//! `cards/artifacts/equipment/mv_2/vorrac_battlehorns.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vorrac_battlehorns() -> CardIndex {
    card_index("89750d72-d1c0-4c8a-aa72-fdd48570aa92")
}

/// Vorrac Battlehorns (89750d72-d1c0-4c8a-aa72-fdd48570aa92) — {2} Equipment:
/// "Equipped creature has trample and can't be blocked by more than one
///  creature."
/// "Equip {1}"
///
/// Equipped 4/5 Jasmine Boreal gains Trample and caps blockers at 1 (double
/// block with two Llanowar Elves is rejected; single block is legal and 3
/// trample damage spills over onto the opponent).
#[test]
fn vorrac_battlehorns_grants_trample_and_caps_blockers_at_one() {
    let p1 = PlayerId::new(1);
    let mut r = rig_vs(
        vorrac_battlehorns(),
        jasmine_boreal(),
        1,
        &[],
        &[llanowar_elves(), llanowar_elves()],
        &[],
    );
    assert_eq!(keywords(&r.engine, r.host), KeywordSet::EMPTY);

    r.equip(1);
    assert_eq!(pt(&r.engine, r.host), (4, 5), "no P/T change");
    assert!(
        keywords(&r.engine, r.host).contains(KeywordSet::TRAMPLE),
        "equipped creature gains trample"
    );
    assert_eq!(
        keywords(&r.engine, r.bystander),
        KeywordSet::EMPTY,
        "unequipped bystander gains no keyword"
    );

    let elves = all_on_battlefield(&r.engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2);

    let Pending::ChooseBlockers { bounds, .. } = r.attack_to_the_blocks() else {
        unreachable!("attack_to_the_blocks returns ChooseBlockers")
    };
    let host_bound = bounds
        .iter()
        .find(|b| b.attacker == r.host)
        .expect("attacker bound for equipped host");
    assert_eq!(
        host_bound.max_blockers, 1,
        "equipped creature can't be blocked by more than one creature"
    );

    // Attempting to double-block is refused by the engine.
    assert!(
        r.engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(elves[0], r.host), (elves[1], r.host)],
                },
            )
            .is_err(),
        "blocking with two creatures violates MaxBlockers(1)"
    );

    // Blocking with one Elf is legal, and 3 excess trample damage hits p1 (20 -> 17).
    r.engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves[0], r.host)],
            },
        )
        .expect("a single blocker is legal");
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        r.engine.state().object(elves[0]).unwrap().zone,
        Zone::Graveyard,
        "single 1/1 blocker died"
    );
    assert_eq!(
        r.engine.state().object(elves[1]).unwrap().zone,
        Zone::Battlefield,
        "second Elf stayed back"
    );
    assert_eq!(
        life_of(&r.engine, p1),
        17,
        "3 excess trample damage hit opponent (4 - 1 = 3, 20 -> 17)"
    );
}
