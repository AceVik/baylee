//! `cards/enchantments/auras/mv_3/aggression.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aggression() -> CardIndex {
    card_index("b6c38a2b-2fe8-4080-80ce-cfa12aaf57f0")
}

fn wall_of_wood() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

/// Aggression (b6c38a2b-2fe8-4080-80ce-cfa12aaf57f0) — {2}{R} Enchantment — Aura:
/// - "Enchant non-Wall creature"
/// - "Enchanted creature has first strike and trample."
/// - "At the beginning of the end step of enchanted creature's controller,
///   destroy that creature if it didn't attack this turn."
#[test]
fn aggression_targets_non_wall_grants_first_strike_and_trample_and_destroys_if_it_did_not_attack() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                gray_ogre(),
                wall_of_wood(),
            ],
        )
        .hand(0, &[aggression()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("Gray Ogre seated");
    let wall = on_battlefield(&engine, P0, wall_of_wood()).expect("Wall of Wood seated");

    cast_from_hand(&mut engine, P0, aggression());
    let menu = pass_until_targets(&mut engine, P0);
    assert!(menu.contains(&ogre), "non-Wall creature is a legal target");
    assert!(
        !menu.contains(&wall),
        "Wall creature is excluded by Enchant non-Wall creature"
    );

    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, ogre);
    assert!(
        kw.contains(KeywordSet::FIRST_STRIKE) && kw.contains(KeywordSet::TRAMPLE),
        "enchanted creature has first strike and trample: {kw:?}"
    );

    // Do not attack with Gray Ogre this turn -> at the beginning of P0's end step,
    // Aggression triggers and destroys Gray Ogre.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_none(),
        "Gray Ogre did not attack during P0's turn and was destroyed at P0's end step"
    );
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
}

#[test]
fn aggression_spares_enchanted_creature_when_it_attacked_this_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), gray_ogre()])
        .hand(0, &[aggression()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("Gray Ogre seated");
    cast_from_hand(&mut engine, P0, aggression());
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    // Attack with Gray Ogre this turn.
    let _blocks = attack_and_collect_blocks(&mut engine, ogre, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);

    // Pass through P0's end step into P1's main phase: Gray Ogre survives.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "Gray Ogre attacked this turn so Aggression does not destroy it"
    );
}

/// "At the beginning of the end step of enchanted creature's controller":
/// P0's Aggression on P1's Ogre. P0's own end step passes it by, though the
/// Ogre did not attack that turn; P1's end step destroys it after P1 does
/// not attack with it.
#[test]
fn aggression_on_an_opponents_creature_waits_for_that_players_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[aggression()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let their_ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre seated");
    cast_from_hand(&mut engine, P0, aggression());
    aim_at(&mut engine, P0, their_ogre);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P1, gray_ogre()).is_some(),
        "P0's end step is not the end step of the Ogre's controller"
    );

    assert!(walk_to_own_main(&mut engine, P0));
    assert!(
        on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "P1's Ogre did not attack on P1's turn and was destroyed at P1's end step"
    );
    assert!(in_graveyard(&engine, P1, gray_ogre()).is_some());
}
