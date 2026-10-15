//! Oracle: Trample / Other creatures you control have trample.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("e75bc5df-f4a4-45af-a6fd-2711b98dd521")
}

// Checklist: play source; own recipient, self inclusion/exclusion, opponent and
// nonmatching bystanders; exact stats/keyword; real bounce ends continuous grant.
#[test]
fn grant_obeys_recipients_and_ends_when_source_leaves() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                forest(),
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE, index::GRAY_OGRE])
        .hand(0, &[subject(), index::GRAY_OGRE, unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, index::GRAY_OGRE);
    pass_until(&mut e, stack_is_empty);
    let ally = on_battlefield(&e, p0, index::GRAY_OGRE).unwrap();
    let foe = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    assert_eq!(pt(&e, ally), (2, 2));
    cast_with_floating(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let source = on_battlefield(&e, p0, subject()).unwrap();
    assert_eq!(pt(&e, source), (3, 3));
    assert_eq!(pt(&e, ally), (2, 2));
    assert_eq!(pt(&e, foe), (2, 2));
    assert!(keywords(&e, ally).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&e, source).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, foe).contains(KeywordSet::TRAMPLE));
    cast_with_floating(&mut e, p0, unsummon());
    aim_at(&mut e, p0, source);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, subject()).is_some());
    assert_eq!(pt(&e, ally), (2, 2));
    assert!(!keywords(&e, ally).contains(KeywordSet::TRAMPLE));
}

// Combat proof: granted keyword changes actual blocked combat damage; for sources
// with an intrinsic keyword, exercise the source separately as well.
#[test]
fn keyword_changes_blocked_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for use_source in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[subject(), index::GRAY_OGRE])
            .battlefield(1, &[index::METALLIC_SLIVER])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let card = if use_source {
            subject()
        } else {
            index::GRAY_OGRE
        };
        let attacker = on_battlefield(&e, p0, card).unwrap();
        let blocker = on_battlefield(&e, p1, index::METALLIC_SLIVER).unwrap();
        attack_and_collect_blocks(&mut e, attacker, p1);
        let before = e.state().players[1].life;
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.state().turn.phase, Phase::SecondMain)
        });
        let expected = if use_source { 2 } else { 1 };
        assert_eq!(e.state().players[1].life, before - expected);
        assert!(on_battlefield(&e, p0, card).is_some());
        assert!(in_graveyard(&e, p1, index::METALLIC_SLIVER).is_some());
    }
}
