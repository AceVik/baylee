//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn tetsuo_refuses_auras_and_destroys_only_tapped_or_blocking_creatures() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[index::TETSUO_UMEZAWA, grizzly_bears(), llanowar_elves()],
        )
        .hand(0, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tetsuo = on_battlefield(&e, p0, index::TETSUO_UMEZAWA).unwrap();
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    give_mana(&mut e, p0, ManaColor::White, 1);
    let aura = in_hand(&e, p0, index::HOLY_STRENGTH).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: aura }).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(!options.contains(&tetsuo));
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    activate(&mut e, p0, llanowar_elves(), 0);
    give_mana(&mut e, p0, ManaColor::Blue, 1);
    give_mana(&mut e, p0, ManaColor::Black, 2);
    give_mana(&mut e, p0, ManaColor::Red, 1);
    activate(&mut e, p0, index::TETSUO_UMEZAWA, 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(options.contains(&elf));
    assert!(!options.contains(&bear));
    aim_at(&mut e, p0, elf);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(elf).unwrap().zone, Zone::Graveyard);
}
