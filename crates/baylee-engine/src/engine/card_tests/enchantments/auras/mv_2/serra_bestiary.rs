//! Serra Bestiary — `{W}{W}` Enchantment — Aura (Homelands)
//!
//! Enchant creature
//! At the beginning of your upkeep, sacrifice this Aura unless you pay `{W}{W}`.
//! Enchanted creature can't attack or block, and its activated abilities with `{T}` in their costs can't be activated.

#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_bestiary() -> CardIndex {
    card_index("320d8f2b-005d-4769-80d2-f46d96afffe7")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

#[test]
fn serra_bestiary_prevents_attack_block_and_activation_and_sacrifices_at_upkeep_if_unpaid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[serra_bestiary()])
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[prodigal_sorcerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bestiary_card = in_hand(&engine, p0, serra_bestiary()).unwrap();
    let tim = on_battlefield(&engine, p1, prodigal_sorcerer()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: bestiary_card,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![tim],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to p1's FirstMain: Prodigal Sorcerer can't activate its {T} ability and can't attack.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ActivateAbility {
                    source: tim,
                    ability_index: 0,
                },
            )
            .is_err(),
        "enchanted Prodigal Sorcerer cannot activate its ability"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(tim, Defender::Player(p0))],
                },
            )
            .is_err(),
        "enchanted Prodigal Sorcerer cannot attack"
    );
    engine
        .apply(p1, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // At p0's next upkeep, decline to pay {W}{W}: Serra Bestiary is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, p0, serra_bestiary()).is_none(),
        "Serra Bestiary was sacrificed when {{W}}{{W}} was not paid"
    );
}
