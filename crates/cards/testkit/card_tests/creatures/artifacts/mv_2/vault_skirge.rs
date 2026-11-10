//! `cards/creatures/artifacts/mv_2/vault_skirge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vault_skirge() -> CardIndex {
    card_index("64cf5d59-7bcd-4b0b-a160-c8468d4c0f60")
}

/// Vault Skirge: a 1/1 with "Flying" and "Lifelink", `{1}{B/P}`. Attacking
/// unblocked it deals one and its controller gains one.
#[test]
fn vault_skirge_flies_and_gains_life_by_lifelink() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[vault_skirge()], &[grizzly_bears(), storm_crow()]);
    let skirge = on_battlefield(&engine, p0, vault_skirge()).expect("seated");
    assert_eq!(pt(&engine, skirge), (1, 1));
    assert!(keywords(&engine, skirge).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, skirge).contains(KeywordSet::LIFELINK));
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, skirge, p1);
    assert!(!options.iter().any(|o| o.blocker == bears), "flying");
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 19, "one damage dealt");
    assert_eq!(life_of(&engine, 0), 21, "lifelink: one life gained");
}

/// `{B/P}` can be paid with `{B}`: a Swamp and a Forest cast it and no life
/// is lost.
#[test]
fn vault_skirge_is_cast_for_one_and_black_without_losing_life() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[vault_skirge()], &[swamp(), forest()], &[]);
    cast_from_hand(&mut engine, p0, vault_skirge());
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, vault_skirge()).is_some());
    assert_eq!(life_of(&engine, 0), 20, "paid in mana, not in life");
}

/// `{B/P}` can be paid with 2 life: two Forests pay the `{1}` and two life
/// the Phyrexian symbol.
#[test]
fn vault_skirge_is_cast_without_black_by_paying_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[vault_skirge()], &[forest(), forest()], &[]);
    cast_from_hand(&mut engine, p0, vault_skirge());
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, vault_skirge()).is_some());
    assert_eq!(life_of(&engine, 0), 18, "two life for the Phyrexian symbol");
}
