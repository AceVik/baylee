#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
fn song_of_serenity() -> CardIndex {
    card_index("4c543799-1b72-4f32-a5be-475c0134ac0f")
}
fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}
#[test]
fn song_of_serenity_excludes_enchanted_creatures_from_attackers_and_blockers_for_both_sides() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), gray_ogre()],
        &[grizzly_bears(), gray_ogre()],
        &[song_of_serenity(), holy_strength(), holy_strength()],
        &[],
    );
    let mine = lk::obj(&e, p0, grizzly_bears());
    let free_mine = lk::obj(&e, p0, gray_ogre());
    let theirs = lk::obj(&e, p1, grizzly_bears());
    let free_theirs = lk::obj(&e, p1, gray_ogre());
    lk::float(
        &mut e,
        p0,
        &[(ManaColor::Green, 10), (ManaColor::White, 10)],
    );
    lk::cast_at(&mut e, p0, song_of_serenity(), &[], &[]);
    lk::cast_at(&mut e, p0, holy_strength(), &[mine], &[]);
    lk::cast_at(&mut e, p0, holy_strength(), &[theirs], &[]);
    let attacks = lk::attackers_offered(&mut e);
    assert!(!attacks.contains(&mine));
    assert!(attacks.contains(&free_mine));
    e.apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut e, p1);
    let attacks = lk::attackers_offered(&mut e);
    assert!(!attacks.contains(&theirs));
    assert!(attacks.contains(&free_theirs));
    let blocks = lk::declare(&mut e, &[free_theirs]);
    assert!(!lk::may_block(&blocks, mine, free_theirs));
    assert!(lk::may_block(&blocks, free_mine, free_theirs));
}
