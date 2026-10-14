//! `cards/artifacts/equipment/mv_1/avacyn_s_collar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn avacyn_s_collar() -> CardIndex {
    card_index("842e3835-fff3-4684-bfd1-e7064a1fd2e6")
}

/// Avacyn's Collar — "Equipped creature gets +1/+0 and has vigilance.
/// Whenever equipped creature dies, if it was a Human, create a 1/1 white
/// Spirit creature token with flying." and "Equip {2}". A Human host dies and
/// leaves a 1/1 flying Spirit.
#[test]
fn avacyn_s_collar_leaves_a_spirit_when_its_human_dies() {
    let (poet, elf) = (tragic_poet(), llanowar_elves());
    let mut engine = rig(2, &[], &[avacyn_s_collar(), poet, elf], &[]);
    let p0 = PlayerId::new(0);
    let human = on_battlefield(&engine, p0, poet).expect("the Human");
    let bystander = on_battlefield(&engine, p0, elf).expect("the Elf");
    equip(&mut engine, avacyn_s_collar(), 2, human);
    assert_eq!(pt(&engine, human), (2, 1), "+1/+0");
    assert!(keywords(&engine, human).contains(KeywordSet::VIGILANCE));
    assert!(!keywords(&engine, bystander).contains(KeywordSet::VIGILANCE));
    assert_eq!(pt(&engine, bystander), (1, 1));
    assert!(tokens_of(&engine, p0).is_empty(), "no Spirit yet");

    kill(&mut engine, human);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Spirit");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}

/// "If it was a Human": a non-Human host dying leaves nothing behind.
#[test]
fn avacyn_s_collar_leaves_nothing_when_a_non_human_dies() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[], &[avacyn_s_collar(), elf], &[]);
    let p0 = PlayerId::new(0);
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    equip(&mut engine, avacyn_s_collar(), 2, host);
    assert_eq!(pt(&engine, host), (2, 1));
    kill(&mut engine, host);
    assert!(tokens_of(&engine, p0).is_empty(), "an Elf is no Human");
    assert!(on_battlefield(&engine, p0, avacyn_s_collar()).is_some());
}
