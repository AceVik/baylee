//! Vintara Elephant — {4}{G} — Creature — Elephant
//! Oracle: Trample
//! Oracle: {3}: This creature loses trample until end of turn. Any player may activate this ability.

use super::*;

#[test]
fn any_player_may_pay_three_to_strip_vintara_elephants_trample() {
    let vintara_elephant = card_index("0a73b4da-9b5c-480e-b58b-6a82a52ea71e");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[vintara_elephant])
        .battlefield(1, &[plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, vintara_elephant);
    pass_until(&mut engine, stack_is_empty);

    let elephant_id =
        on_battlefield(&engine, p0, vintara_elephant).expect("Vintara Elephant on board");
    assert!(keywords(&engine, elephant_id).contains(KeywordSet::TRAMPLE));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    assert!(
        priority_offer(&engine)
            .abilities
            .contains(&(elephant_id, 0))
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: elephant_id,
                ability_index: 0,
            },
        )
        .expect("opponent activates Vintara Elephant ability");
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, elephant_id).contains(KeywordSet::TRAMPLE));

    assert!(walk_to_own_main(&mut engine, p1));
    assert!(keywords(&engine, elephant_id).contains(KeywordSet::TRAMPLE));
}
