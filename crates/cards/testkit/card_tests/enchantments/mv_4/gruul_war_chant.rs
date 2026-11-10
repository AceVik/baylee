//! `cards/enchantments/mv_4/gruul_war_chant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gruul_war_chant() -> CardIndex {
    card_index("a9edb891-1a38-4b8a-8539-c459e8a52315")
}

/// "Attacking creatures you control get +1/+0 and have menace.": the
/// attacker is a 3/2 with menace, the one that stays home is a plain 2/2,
/// and before the attack neither has it.
#[test]
fn gruul_war_chant_pumps_and_menaces_my_attackers_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let menace = baylee_cards_dsl::KeywordSet::MENACE;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gruul_war_chant(), grizzly_bears(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    assert_eq!(pt(&engine, bears), (2, 2), "idle: as printed");
    assert!(!keywords(&engine, bears).contains(menace));

    attack_with(&mut engine, bears, p1);
    assert_eq!(pt(&engine, bears), (3, 2), "attacking: +1/+0");
    assert!(keywords(&engine, bears).contains(menace), "and menace");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the creature at home is unchanged"
    );
    assert!(!keywords(&engine, elf).contains(menace));
}
