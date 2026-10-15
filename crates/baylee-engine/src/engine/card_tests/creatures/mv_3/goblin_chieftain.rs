//! `cards/creatures/mv_3/goblin_chieftain.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn goblin_chieftain() -> CardIndex {
    card_index("368b4052-174e-4458-a6e6-eaf8093aa0fe")
}

// Checklist: Chieftain's own haste works on the turn it enters; only other
// Goblins you control get +1/+1 and haste; opponent Goblin and own non-Goblin do not.
#[test]
fn goblin_chieftain_and_its_other_goblins_can_attack_immediately() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
                swamp(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(
            0,
            &[
                goblin_chieftain(),
                festering_goblin(),
                index::LLANOWAR_ELVES,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, festering_goblin());
    pass_until(&mut e, stack_is_empty);
    cast_from_hand(&mut e, p0, index::LLANOWAR_ELVES);
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, p0, goblin_chieftain());
    pass_until(&mut e, stack_is_empty);

    let chief = on_battlefield(&e, p0, goblin_chieftain()).expect("Chieftain entered");
    let goblin = on_battlefield(&e, p0, festering_goblin()).expect("our other Goblin");
    let elf = on_battlefield(&e, p0, llanowar_elves()).expect("non-Goblin");
    let their_goblin = on_battlefield(&e, p1, festering_goblin()).expect("opponent's Goblin");
    assert_eq!(pt(&e, chief), (2, 2), "other Goblins excludes itself");
    assert!(keywords(&e, chief).contains(KeywordSet::HASTE));
    assert_eq!(pt(&e, goblin), (2, 2));
    assert!(keywords(&e, goblin).contains(KeywordSet::HASTE));
    assert_eq!(pt(&e, elf), (1, 1));
    assert!(!keywords(&e, elf).contains(KeywordSet::HASTE));
    assert_eq!(pt(&e, their_goblin), (1, 1));
    assert!(!keywords(&e, their_goblin).contains(KeywordSet::HASTE));

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("the turn reaches declare attackers")
    };
    assert!(
        attackers.contains(&chief),
        "the newly cast Chieftain has haste"
    );
    assert!(attackers.contains(&goblin), "the other Goblin gains haste");
    assert!(
        !attackers.contains(&elf),
        "the summoning-sick Elf did not gain haste"
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (chief, Defender::Player(p1)),
                (goblin, Defender::Player(p1)),
            ],
        },
    )
    .expect("both haste-enabled Goblins may attack now");
}
