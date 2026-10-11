//! Mikaeus grants an actual death trigger and retaliates against Human damage.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn mikaeus_grants_undying_to_other_nonhumans_and_only_once_without_counters() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[mikaeus_the_unhallowed(), llanowar_elves(), royal_assassin()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, mikaeus_the_unhallowed()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let human = on_battlefield(&e, p0, royal_assassin()).unwrap();
    assert!(keywords(&e, source).contains(KeywordSet::INTIMIDATE));
    assert!(!keywords(&e, source).contains(KeywordSet::UNDYING));
    assert!(!keywords(&e, human).contains(KeywordSet::UNDYING));
    assert!(keywords(&e, elf).contains(KeywordSet::UNDYING));
    assert_eq!(pt(&e, elf), (2, 2));
    for returns in [true, false] {
        crate::sba::destroy(e.dev_state_mut(p0).unwrap(), elf);
        let Pending::Priority { player, .. } = e.pending().clone() else {
            panic!("priority")
        };
        e.apply(player, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(on_battlefield(&e, p0, llanowar_elves()).is_some(), returns);
        if returns {
            assert_eq!(counters_on(&e, elf, CounterKind::P1P1), 1);
        }
    }
}
