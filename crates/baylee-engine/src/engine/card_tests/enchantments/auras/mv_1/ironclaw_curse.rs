//! Ironclaw Curse — `{R}` Enchantment — Aura (Homelands)
//!
//! Enchant creature
//! Enchanted creature gets -0/-1.
//! Enchanted creature can't block creatures with power equal to or greater than
//! the enchanted creature's toughness.
//!
//! (`Coverage::Partial`: blocking restriction comparing attacker power to
//! enchanted creature's toughness is not in `Filter`; enchanted creature gets
//! -0/-1.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn ironclaw_curse() -> CardIndex {
    card_index("51708bd1-23db-4597-84a7-f36abd93c537")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn ironclaw_curse_gives_enchanted_creature_minus_zero_minus_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[ironclaw_curse()])
        .battlefield(0, &[mountain(), bears()])
        .battlefield(1, &[bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let curse = in_hand(&engine, p0, ironclaw_curse()).unwrap();
    let target = on_battlefield(&engine, p0, bears()).unwrap();
    let bystander = on_battlefield(&engine, PlayerId::new(1), bears()).unwrap();
    assert_eq!(pt(&engine, target), (2, 2));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: curse })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, target), (2, 1));
    assert_eq!(
        pt(&engine, bystander),
        (2, 2),
        "only the enchanted creature"
    );
}
