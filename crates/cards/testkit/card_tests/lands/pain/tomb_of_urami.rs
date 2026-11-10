//! `cards/lands/pain/tomb_of_urami.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tomb of Urami prints `{{T}}: Add {{B}}. Tomb of Urami deals 1 damage to you if you don't control an Ogre.`
/// and `{2}{B}{B}, {{T}}, Sacrifice all lands you control: Create Urami, a legendary 5/5 black Demon Spirit creature token with flying.`
///
/// Under `Coverage::Partial`, the Urami ability is omitted because its sacrifice-all-lands cost
/// cannot be stated. The mana ability carries its rider: tapped with no Ogre, the land makes one
/// black mana and deals 1 damage to its controller; tapped beside Ogre Berserker, it deals none.
/// Only one ability is offered either way.
#[test]
fn tomb_of_urami_hurts_without_an_ogre_and_not_with_one() {
    let p0 = PlayerId::new(0);
    let life_lost = |board: &[CardIndex]| {
        let mut engine = Duel::new(SEED, forest()).battlefield(0, board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let tomb = on_battlefield(&engine, p0, tomb_of_urami()).expect("tomb on battlefield");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert_eq!(
            legal.abilities.iter().filter(|(id, _)| *id == tomb).count(),
            1,
            "only the mana ability: the Urami ability is not written"
        );

        let before = engine.state().players[0].life;
        activate(&mut engine, p0, tomb_of_urami(), 0);
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Black),
            1,
            "{{T}}: Add {{B}}"
        );
        assert!(is_tapped(&engine, tomb));
        before - engine.state().players[0].life
    };

    assert_eq!(life_lost(&[tomb_of_urami()]), 1, "no Ogre: 1 damage");
    assert_eq!(
        life_lost(&[tomb_of_urami(), ogre_berserker()]),
        0,
        "an Ogre stops it"
    );
}
