//! Willow Priestess — `{2}{G}{G}` 2/2 Creature — Faerie Druid (Homelands)
//!
//! `{T}`: You may put a Faerie permanent card from your hand onto the battlefield.
//! `{2}{G}`: Target green creature gains protection from black until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn willow_priestess() -> CardIndex {
    card_index("630eab2b-c1fd-46bb-8ae0-ff837e46db1c")
}

fn scryb_sprites() -> CardIndex {
    card_index("a1f20695-6f08-4d5c-9fba-b0018bee298e")
}

fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

#[test]
fn willow_priestess_puts_faerie_from_hand_and_grants_green_creature_protection_from_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[scryb_sprites()])
        .hand(1, &[terror()])
        .battlefield(0, &[willow_priestess(), forest(), forest(), forest()])
        .battlefield(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let priestess = on_battlefield(&engine, p0, willow_priestess()).unwrap();
    let sprites_card = in_hand(&engine, p0, scryb_sprites()).unwrap();

    // Activate {T}: put Scryb Sprites from hand onto the battlefield.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: priestess,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sprites_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sprites_bf = on_battlefield(&engine, p0, scryb_sprites()).unwrap();

    // Activate {2}{G}: target green creature (Scryb Sprites) gains protection from black until end of turn.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: priestess,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sprites_bf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Verify p1 cannot target Scryb Sprites with black Terror!
    let terror_card = in_hand(&engine, p1, terror()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: terror_card })
        .unwrap();
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ChooseTargets {
                    objects: vec![sprites_bf],
                    players: vec![],
                },
            )
            .is_err(),
        "Scryb Sprites has protection from black and cannot be targeted by Terror"
    );
}
