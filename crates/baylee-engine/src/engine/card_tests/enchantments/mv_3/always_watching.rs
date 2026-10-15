//! `cards/enchantments/mv_3/always_watching.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn always_watching() -> CardIndex {
    card_index("c2153115-bc12-458d-baeb-538ec61e1242")
}

// Checklist: own nontoken gets +1/+1 and vigilance; own token gets neither;
// opponent creature gets neither; vigilance keeps an attacking creature untapped;
// removing the enchantment ends both continuous effects.
#[test]
fn always_watching_buffs_only_nontoken_creatures_and_grants_working_vigilance() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[always_watching(), index::SPROUT, index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let elf = on_battlefield(&e, p0, llanowar_elves()).expect("our nontoken Elf");
    let plains_on_board = all_on_battlefield(&e, p0, plains());
    assert_eq!(plains_on_board.len(), 5);
    let reserved_white = [plains_on_board[0], plains_on_board[1]];
    tap_mana_where(&mut e, p0, |id| id != elf && !reserved_white.contains(&id));
    cast_with_floating(&mut e, p0, always_watching());
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, p0, index::SPROUT);
    pass_until(&mut e, stack_is_empty);

    let their_elf = on_battlefield(&e, p1, llanowar_elves()).expect("opponent's Elf");
    let token = tokens_of(&e, p0).into_iter().next().expect("Sprout token");
    assert_eq!(pt(&e, elf), (2, 2));
    assert!(keywords(&e, elf).contains(KeywordSet::VIGILANCE));
    assert_eq!(pt(&e, token), (1, 1), "tokens are excluded from the buff");
    assert!(!keywords(&e, token).contains(KeywordSet::VIGILANCE));
    assert_eq!(
        pt(&e, their_elf),
        (1, 1),
        "the effect is limited to your side"
    );
    assert!(!keywords(&e, their_elf).contains(KeywordSet::VIGILANCE));

    let blocks = attack_and_collect_blocks(&mut e, elf, p1);
    assert!(blocks.iter().all(|b| b.attackers.contains(&elf)));
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("the opponent declines to block");
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert!(!is_tapped(&e, elf), "vigilance prevents tapping to attack");

    let anthem = on_battlefield(&e, p0, always_watching()).expect("the source remains");
    cast_from_hand(&mut e, p0, index::DISENCHANT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!("Disenchant chooses its enchantment target")
    };
    assert!(options.contains(&anthem));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![anthem],
        },
    )
    .expect("Always Watching is a legal enchantment target");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, elf), (1, 1), "leaving removes the bonus");
    assert!(!keywords(&e, elf).contains(KeywordSet::VIGILANCE));
}
