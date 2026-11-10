//! Printed rules exercised through real choices and resolution.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn bartel_excludes_aura_spells_but_not_other_spells() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::BARTEL_RUNEAXE, plains(), plains()])
        .hand(0, &[index::HOLY_STRENGTH, swords_to_plowshares()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let aura = in_hand(&e, p0, index::HOLY_STRENGTH).unwrap();
    let swords = in_hand(&e, p0, swords_to_plowshares()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&aura));
    assert!(legal.castable.contains(&swords));
    let bartel = on_battlefield(&e, p0, index::BARTEL_RUNEAXE).unwrap();
    assert!(keywords(&e, bartel).contains(KeywordSet::VIGILANCE));
    e.apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();
    aim_at(&mut e, p0, bartel);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(bartel).unwrap().zone, Zone::Exile);
}
