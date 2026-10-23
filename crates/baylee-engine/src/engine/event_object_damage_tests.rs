//! Damage dealt to "that creature" by a trigger that remembers it.
//!
//! A delayed trigger is about an object nobody targets (CR 603.7c): Dwarven
//! Sea Clan targets an attacker as it is activated, and the trigger it
//! leaves behind deals 2 damage to that creature at end of combat with no
//! target of its own. `Effect::DealDamage { target: EventObject }` read
//! only the chosen targets, so the delayed trigger resolved and dealt
//! nothing. The moving effects (Goblin Sappers' destroy) already read the
//! remembered object through `zones::spec_object`; damage asks it too now.

use super::testkit::{
    Duel, card_index, keep_mulligans, on_battlefield, pass_until, reach_main_phase, stack_is_empty,
};
use super::*;
use crate::zone::Zone;
use baylee_core::ids::{CardIndex, Defender};

const SEED: u64 = 0x5ea_c1a0;

/// "{T}: Choose target attacking or blocking creature whose controller
/// controls an Island. This creature deals 2 damage to that creature at end
/// of combat. Activate only before the end of combat step."
fn dwarven_sea_clan() -> CardIndex {
    card_index("55ae9711-65ac-4166-8a51-1deeb10a8904")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// The attacker survives the ability's resolution and its own combat damage
/// step, and dies to the 2 damage when the end of combat comes.
#[test]
fn a_delayed_trigger_deals_its_damage_to_the_creature_it_remembers() {
    let (us, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[dwarven_sea_clan()])
        .battlefield(1, &[grizzly_bears(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, us);
    let clan = on_battlefield(&engine, us, dwarven_sea_clan()).unwrap();
    let bears = on_battlefield(&engine, them, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == them && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            them,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(us))],
            },
        )
        .unwrap();
    engine.apply(them, PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            us,
            PlayerAction::ActivateAbility {
                source: clan,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            us,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        0,
        "the damage waits for the end of combat"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        18,
        "the attacker lived to deal its combat damage"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Graveyard,
        "the delayed trigger dealt its 2 damage to the attacker it remembers"
    );
}
