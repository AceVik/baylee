//! `cards/artifacts/equipment/mv_2/warmonger_s_chariot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn warmonger_s_chariot() -> CardIndex {
    card_index("90f4ec87-d33d-46c1-8cc7-3e7ad3f8ad70")
}

/// Warmonger's Chariot: "Equipped creature gets +2/+2. As long as equipped
/// creature has defender, it can attack as though it didn't have defender.
/// Equip {3}".
#[test]
fn warmonger_s_chariot_lets_a_defender_attack() {
    let attackers_offered = |r: &mut Rig| {
        pass_until(&mut r.engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers { attackers, .. } = r.engine.pending().clone() else {
            unreachable!("pass_until stops on the question")
        };
        attackers
    };
    // Without the Chariot the Wall is not offered.
    let mut bare = rig(warmonger_s_chariot(), steel_wall(), 3);
    let offered = attackers_offered(&mut bare);
    assert!(
        !offered.contains(&bare.host),
        "a defender cannot attack: {offered:?}"
    );
    // With it, it is, and it is a 2/6.
    let mut r = rig(warmonger_s_chariot(), steel_wall(), 3);
    r.equip(3);
    assert_eq!(pt(&r.engine, r.host), (2, 6), "+2/+2");
    let offered = attackers_offered(&mut r);
    assert!(
        offered.contains(&r.host),
        "the equipped defender may attack: {offered:?}"
    );
}

#[test]
fn warmonger_s_chariot_pumps_any_creature() {
    equipped_gets(
        warmonger_s_chariot(),
        3,
        llanowar_elves(),
        (2, 2),
        KeywordSet::EMPTY,
    );
}
