//! `cards/creatures/mv_3/crypt_rats.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn crypt_rats() -> CardIndex {
    card_index("104095ed-55e3-408e-bf70-4fe06bb16d2f")
}

/// Crypt Rats (`{2}{B}`, `Coverage::Partial`): only the 1/1 body is played.
/// The `{X}` damage ability is left off because "spend only black mana on X"
/// cannot be said for an activated ability, and with any mana paying X it
/// would be stronger than printed: with three Swamps untapped the ability must
/// not be offered, activating it is refused, and nothing is damaged.
#[test]
fn crypt_rats_is_a_1_1_body_whose_damage_ability_is_not_played() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[crypt_rats()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Three Swamps pay for the Rats; three stay untapped, ready to pay an X.
    let spend: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(3).collect();
    tap_mana_where(&mut engine, p0, |land| spend.contains(&land));
    cast_with_floating(&mut engine, p0, crypt_rats());
    pass_until(&mut engine, stack_is_empty);
    let rats = on_battlefield(&engine, p0, crypt_rats()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, rats), (1, 1), "the printed body");

    assert!(
        !ability_offered(&engine, rats, 0),
        "the {{X}} ability is not played"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: rats,
                    ability_index: 0,
                },
            )
            .is_err(),
        "there is no ability to activate"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(bear).unwrap().damage, 0);
    assert!(on_battlefield(&engine, p0, crypt_rats()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
