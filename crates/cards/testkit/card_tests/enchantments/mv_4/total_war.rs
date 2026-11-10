//! Total War — `{3}{R}` Enchantment (Ice Age)
//!
//! Whenever one or more creatures attack, destroy all untapped non-Wall
//! creatures that player controls that didn't attack, except for creatures the
//! player hasn't controlled continuously since the beginning of the turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn total_war() -> CardIndex {
    card_index("8266dd1c-e262-4467-a893-6b3b5eedb427")
}

fn wall_of_bone() -> CardIndex {
    card_index("8ffe4986-9e09-4421-ad26-296a4c0df9e4")
}

fn tundra_wolves() -> CardIndex {
    card_index("2155864e-5788-4e72-a800-e2cf25cf59a7")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

#[test]
fn total_war_destroys_only_untapped_non_wall_non_attackers_controlled_since_turn_began() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[total_war(), tundra_wolves()])
        .battlefield(
            0,
            &[
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),     // will attack -> survives
                hill_giant(),        // stays back untapped -> destroyed
                prodigal_sorcerer(), // tapped before combat -> survives
                wall_of_bone(),      // untapped Wall -> survives
            ],
        )
        .battlefield(1, &[grizzly_bears()]) // opponent's untapped non-Wall -> survives
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let shirker = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let tapped_ally = on_battlefield(&engine, p0, prodigal_sorcerer()).unwrap();
    let wall = on_battlefield(&engine, p0, wall_of_bone()).unwrap();
    let opp_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Cast Total War ({3}{R}) and Tundra Wolves ({W}, newly controlled this turn).
    let tw_hand = in_hand(&engine, p0, total_war()).unwrap();
    let wolves_hand = in_hand(&engine, p0, tundra_wolves()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: tw_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    engine
        .apply(p0, PlayerAction::CastSpell { card: wolves_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let newcomer = on_battlefield(&engine, p0, tundra_wolves()).unwrap();

    // Tap Prodigal Sorcerer before combat so it is tapped when attackers are declared.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tapped_ally,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Declare only Grizzly Bears as an attacker.
    attack_with(&mut engine, attacker, p1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(shirker).unwrap().zone,
        Zone::Graveyard,
        "untapped non-Wall controlled since turn began that didn't attack is destroyed"
    );
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Battlefield,
        "attacking creature survives Total War"
    );
    assert_eq!(
        engine.state().object(tapped_ally).unwrap().zone,
        Zone::Battlefield,
        "tapped creature survives Total War"
    );
    assert_eq!(
        engine.state().object(wall).unwrap().zone,
        Zone::Battlefield,
        "untapped Wall survives Total War"
    );
    assert_eq!(
        engine.state().object(newcomer).unwrap().zone,
        Zone::Battlefield,
        "creature not controlled since the beginning of the turn survives Total War"
    );
    assert_eq!(
        engine.state().object(opp_bears).unwrap().zone,
        Zone::Battlefield,
        "defending player's creature survives Total War"
    );
}

/// "That player" is the one who attacks, not the one who controls Total War:
/// when the opponent attacks with one creature, their idle creature is
/// destroyed and Total War's controller keeps theirs.
#[test]
fn total_war_destroys_only_the_attacking_players_idle_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[total_war(), gray_ogre()]) // idle, but not the attacker's
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);

    let own_idle = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let attacker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let shirker = on_battlefield(&engine, p1, hill_giant()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(shirker).unwrap().zone,
        Zone::Graveyard,
        "the attacking player's untapped non-attacker is destroyed"
    );
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Battlefield,
        "the attacker survives"
    );
    assert_eq!(
        engine.state().object(own_idle).unwrap().zone,
        Zone::Battlefield,
        "Total War's controller did not attack, and their creatures are not 'that player's'"
    );
}
