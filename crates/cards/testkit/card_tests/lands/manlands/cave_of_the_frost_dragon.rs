//! `cards/lands/manlands/cave_of_the_frost_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cave of the Frost Dragon prints the same bound as its complement — "if
/// you control two or more other lands, this land enters tapped" — which is
/// one lower, and that off-by-one is the whole reading.
///
/// The eleventh card the upper bound finished, and the only one of the five
/// manlands whose animation the transcoder could also read: the other four
/// print a ward cost or a trigger no rule says. So it is played twice here,
/// once for the bound and once for the sentence underneath it, because a
/// card that arrives correctly and animates into the wrong thing is still a
/// wrong card.
#[test]
fn a_manland_bound_is_one_lower_and_its_dragon_is_still_a_land() {
    let p0 = PlayerId::new(0);
    let cave = card_index("1e4146d2-cfa0-4f5e-9761-3c83519b90c3");
    assert!(
        !arrives_tapped(
            || Duel::new(1060, forest()).battlefield(0, &[forest()]),
            cave
        ),
        "one other land is not two, and the Cave should have entered untapped"
    );
    assert!(
        arrives_tapped(
            || Duel::new(1061, forest()).battlefield(0, &[forest(), forest()]),
            cave
        ),
        "two other lands is exactly what the card names"
    );

    // {4}{W}: a 3/4 white Dragon with flying, and still a land.
    let mut engine = Duel::new(1062, forest())
        // Five other lands, so the Cave itself arrives tapped — which the
        // animation does not care about, because "{4}{W}:" charges mana and
        // not a tap. The Plains is the white half of that price: the Cave's
        // own mana ability is the card's only other white source and a tapped
        // land cannot pay.
        .battlefield(0, &[cave, plains(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let id = on_battlefield(&engine, p0, cave).expect("the Cave is on the battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(object, index)| *object == id && *index == 1)
        .expect("the animate ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(id)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    let types = engine
        .state()
        .object(id)
        .expect("the Cave exists")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::CREATURE),
        "it never became a Dragon"
    );
    assert!(types.contains(TypeSet::LAND), "\"It's still a land\"");
    assert_eq!(pt(&engine, id), (3, 4), "a 3/4 Dragon");
}
