#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
fn fighting_chance() -> CardIndex {
    card_index("fece632b-d8d5-4d3d-a4a7-643293b67538")
}
#[test]
fn fighting_chance_prevents_a_winning_blockers_damage_but_not_the_attackers_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut saw_win, mut saw_loss) = (false, false);
    for seed in 0..24 {
        let mut e = lk::game(
            seed,
            &[grizzly_bears()],
            &[gray_ogre()],
            &[fighting_chance()],
            &[],
        );
        let attacker = lk::obj(&e, p0, grizzly_bears());
        let blocker = lk::obj(&e, p1, gray_ogre());
        lk::attack(&mut e, &[attacker]);
        lk::block(&mut e, &[(blocker, attacker)]);
        lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
        lk::cast_at(&mut e, p0, fighting_chance(), &[], &[]);
        lk::through_combat(&mut e);
        assert!(
            in_graveyard(&e, p1, gray_ogre()).is_some(),
            "attacker still deals damage"
        );
        if on_battlefield(&e, p0, grizzly_bears()).is_some() {
            saw_win = true;
            assert_eq!(lk::damage_on(&e, attacker), 0);
        } else {
            saw_loss = true;
            assert!(in_graveyard(&e, p0, grizzly_bears()).is_some());
        }
    }
    assert!(saw_win && saw_loss, "exercise both seeded coin outcomes");
}
