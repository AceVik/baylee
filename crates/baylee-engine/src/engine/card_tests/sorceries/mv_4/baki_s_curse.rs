//! Baki's Curse — `{2}{U}{U}` Sorcery (Homelands)
//!
//! Baki's Curse deals 2 damage to each creature for each Aura attached to that
//! creature.
//!
//! (`Coverage::Partial`: per-creature damage scaled by attached Aura count is
//! not in `Effect`; Baki's Curse deals 2 damage to each enchanted creature.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn baki_s_curse() -> CardIndex {
    card_index("341a92b9-d6a0-499e-abb7-e2b7d7bc248c")
}

fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn baki_s_curse_deals_two_damage_to_each_enchanted_creature_and_spares_unenchanted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[holy_strength(), baki_s_curse()])
        .battlefield(
            0,
            &[
                plains(),
                island(),
                island(),
                island(),
                island(),
                bears(), // will be enchanted by Holy Strength (becomes 3/4)
            ],
        )
        .battlefield(1, &[bears()]) // unenchanted 2/2
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enchanted_bears = on_battlefield(&engine, p0, bears()).unwrap();
    let plain_bears = on_battlefield(&engine, PlayerId::new(1), bears()).unwrap();
    let aura_hand = in_hand(&engine, p0, holy_strength()).unwrap();
    let curse_hand = in_hand(&engine, p0, baki_s_curse()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: aura_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![enchanted_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, enchanted_bears), (3, 4));

    engine
        .apply(p0, PlayerAction::CastSpell { card: curse_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(enchanted_bears).unwrap().damage,
        2,
        "enchanted creature takes 2 damage from Baki's Curse"
    );
    assert_eq!(
        engine.state().object(plain_bears).unwrap().damage,
        0,
        "unenchanted creature takes no damage from Baki's Curse"
    );
}
