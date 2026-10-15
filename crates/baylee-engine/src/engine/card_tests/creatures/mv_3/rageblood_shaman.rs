//! Oracle checklist: source has printed trample but is not self-pumped; other own Minotaurs gain +1/+1 and trample; opposing Minotaurs and own non-Minotaurs stay unchanged; two Shamans pump each other.
#[allow(clippy::wildcard_imports)]
use super::*;

fn rageblood_shaman() -> CardIndex {
    card_index("2608f905-a678-4e40-9e73-cf1c42ec880d")
}

#[test]
fn shaman_grants_both_modifiers_to_other_minotaurs_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[rageblood_shaman(), talruum_minotaur(), llanowar_elves()],
        )
        .battlefield(1, &[talruum_minotaur()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let source = on_battlefield(&e, p0, rageblood_shaman()).unwrap();
    let own_minotaur = on_battlefield(&e, p0, talruum_minotaur()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let enemy_minotaur = on_battlefield(&e, p1, talruum_minotaur()).unwrap();
    assert_eq!(
        pt(&e, source),
        (2, 3),
        "'other' excludes the source from its own pump"
    );
    assert!(
        keywords(&e, source).contains(KeywordSet::TRAMPLE),
        "trample is printed on the source"
    );
    assert_eq!(pt(&e, own_minotaur), (4, 4));
    assert!(keywords(&e, own_minotaur).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&e, elf), (1, 1));
    assert!(
        !keywords(&e, elf).contains(KeywordSet::TRAMPLE),
        "the own Elf is not a Minotaur"
    );
    assert_eq!(pt(&e, enemy_minotaur), (3, 3));
    assert!(!keywords(&e, enemy_minotaur).contains(KeywordSet::TRAMPLE));
}

#[test]
fn a_pair_of_shamans_each_counts_as_the_others_other_minotaur() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[rageblood_shaman(), rageblood_shaman()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    for shaman in all_on_battlefield(&e, p0, rageblood_shaman()) {
        assert_eq!(pt(&e, shaman), (3, 4));
        assert!(keywords(&e, shaman).contains(KeywordSet::TRAMPLE));
    }
}
