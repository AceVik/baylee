//! Zerapa Minotaur — {2}{R}{R} — Creature — Minotaur
//! Oracle: First strike
//! Oracle: {2}: This creature loses first strike until end of turn. Any player may activate this ability.

use super::*;

#[test]
fn any_player_may_pay_two_to_strip_zerapa_minotaurs_first_strike() {
    let zerapa_minotaur = card_index("4520c630-3650-462f-ab01-215eb2bdbbf5");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[zerapa_minotaur])
        .battlefield(1, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, zerapa_minotaur);
    pass_until(&mut engine, stack_is_empty);

    let minotaur_id =
        on_battlefield(&engine, p0, zerapa_minotaur).expect("Zerapa Minotaur on board");
    assert!(keywords(&engine, minotaur_id).contains(KeywordSet::FIRST_STRIKE));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    assert!(
        priority_offer(&engine)
            .abilities
            .contains(&(minotaur_id, 0))
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: minotaur_id,
                ability_index: 0,
            },
        )
        .expect("opponent activates Zerapa Minotaur ability");
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, minotaur_id).contains(KeywordSet::FIRST_STRIKE));

    assert!(walk_to_own_main(&mut engine, p1));
    assert!(keywords(&engine, minotaur_id).contains(KeywordSet::FIRST_STRIKE));
}
