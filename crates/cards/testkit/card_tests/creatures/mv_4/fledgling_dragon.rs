//! `cards/creatures/mv_4/fledgling_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn fledgling_dragon_gains_plus_three_plus_three_and_firebreathing_at_threshold() {
    let dragon_card = card_index("be007706-3da7-4dd3-84db-dbb5d8831bba");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
            ],
        )
        .hand(0, &[dragon_card, dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    for m in all_on_battlefield(&engine, p0, mountain())
        .into_iter()
        .take(4)
    {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    let card = in_hand(&engine, p0, dragon_card).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, dragon_card).expect("Fledgling Dragon entered");
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, dragon), (2, 2));

    // Cast Dark Ritual as the 7th card entering the graveyard.
    let s = all_on_battlefield(&engine, p0, swamp())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: s })
        .unwrap();
    let dr = in_hand(&engine, p0, dark_ritual()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: dr })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, dragon), (5, 5));

    // Activate the granted "{R}: This creature gets +1/+0 until end of turn" ability.
    let m_last = all_on_battlefield(&engine, p0, mountain())
        .into_iter()
        .find(|&id| !is_tapped(&engine, id))
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m_last })
        .unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let &(_, ability_index) = legal
        .abilities
        .iter()
        .find(|&&(src, _)| src == dragon)
        .expect("granted firebreathing ability offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (6, 5));
}
