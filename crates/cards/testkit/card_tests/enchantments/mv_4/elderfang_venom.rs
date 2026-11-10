//! `cards/enchantments/mv_4/elderfang_venom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn elderfang_venom() -> CardIndex {
    card_index("4741d842-12e6-4a8c-96b1-9e68c7074874")
}

/// "Attacking Elves you control have deathtouch.": an Elf has it once it
/// attacks and not before; an attacking non-Elf never has it.
#[test]
fn elderfang_venom_gives_deathtouch_to_an_attacking_elf_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let deathtouch = baylee_cards_dsl::KeywordSet::DEATHTOUCH;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elderfang_venom(), llanowar_elves(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert!(
        !keywords(&engine, elf).contains(deathtouch),
        "idle Elf: none"
    );

    attack_with(&mut engine, elf, p1);
    assert!(keywords(&engine, elf).contains(deathtouch), "attacking Elf");
    assert!(
        !keywords(&engine, bears).contains(deathtouch),
        "Bears stay home"
    );
}

/// A Bears that attacks is no Elf.
#[test]
fn elderfang_venom_does_not_touch_an_attacking_non_elf() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elderfang_venom(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    attack_with(&mut engine, bears, p1);
    assert!(
        !keywords(&engine, bears).contains(baylee_cards_dsl::KeywordSet::DEATHTOUCH),
        "an attacking non-Elf has no deathtouch"
    );
}

/// "Whenever an Elf you control dies, each opponent loses 1 life and you gain
/// 1 life.": at three seats both opponents lose 1; a Bears dying does nothing.
#[test]
fn elderfang_venom_drains_each_opponent_when_my_elf_dies() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[elderfang_venom(), llanowar_elves(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");

    kill(&mut engine, bears);
    assert_eq!(life_of(&engine, p0), 20, "a non-Elf dying drains nothing");

    kill(&mut engine, elf);
    assert_eq!(life_of(&engine, p0), 21, "I gain 1");
    assert_eq!(life_of(&engine, p1), 19, "the first opponent loses 1");
    assert_eq!(life_of(&engine, p2), 19, "so does the second");
}
