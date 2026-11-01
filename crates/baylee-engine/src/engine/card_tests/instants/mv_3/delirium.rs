//! `cards/instants/mv_3/delirium.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, STORY_SEEKER};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn delirium() -> CardIndex {
    card_index("a00e5eb6-b916-4cd6-8ff2-c02f4929abc7")
}

/// P0 holds Delirium with `{1}{B}{R}` in play, P1 attacks P0 with `attacker`
/// and P0 has a Grizzly Bears of its own. Returns the engine at P0's priority
/// in P1's declare attackers step, with the attacker and P0's Bears.
fn delirium_in_their_attack(
    attacker: baylee_core::ids::CardIndex,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), swamp(), GRIZZLY_BEARS])
        .battlefield(1, &[attacker])
        .hand(0, &[delirium()])
        .start();
    keep_mulligans(&mut engine);

    let creature = on_battlefield(&engine, P1, attacker).unwrap();
    let mine = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(creature, Defender::Player(P0))],
            },
        )
        .unwrap();

    // P1 gets priority in DeclareAttackers first; pass to P0.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    (engine, creature, mine)
}

/// Delirium (a00e5eb6-b916-4cd6-8ff2-c02f4929abc7) — {1}{B}{R} Instant:
/// "Cast this spell only during an opponent's turn. Tap target creature that
/// player controls. That creature deals damage equal to its power to the
/// player. Prevent all combat damage that would be dealt to and dealt by the
/// creature this turn."
///
/// The attacking Hill Giant is tapped, deals 3 to its controller (P1), and its
/// combat damage to P0 is prevented. The menu is the active player's
/// creatures only: my own Bears are not on it.
#[test]
fn delirium_taps_their_creature_and_makes_it_deal_its_power_to_its_controller() {
    let (mut engine, giant, mine) = delirium_in_their_attack(HILL_GIANT);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, delirium());
    let menu = aim_at(&mut engine, P0, giant);
    assert!(menu.contains(&giant), "{menu:?}");
    assert!(
        !menu.contains(&mine),
        "\"that player's\" creatures only, not my own: {menu:?}"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        engine
            .state()
            .object(giant)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "its combat damage to me was prevented"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "it dealt 3 to its controller"
    );
}

/// "That creature deals damage": the creature is the source, not Delirium. A
/// lifelink Story Seeker (2/2) deals 2 to its controller and gains them 2 back,
/// so P1 ends on 20; with Delirium as the source P1 would be on 18.
#[test]
fn delirium_damage_comes_from_the_creature_so_lifelink_counts() {
    let (mut engine, seeker, _mine) = delirium_in_their_attack(STORY_SEEKER);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, delirium());
    aim_at(&mut engine, P0, seeker);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().players[1].life,
        20,
        "2 damage from a lifelink source, 2 life gained"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

/// "Cast this spell only during an opponent's turn": in my own first main
/// phase, with the mana floating, it is not offered.
#[test]
fn delirium_cannot_be_cast_on_your_own_turn() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[delirium()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, delirium()).unwrap();
    let legal = priority_offer(&engine);
    assert!(
        !legal.castable.contains(&card) && !legal.payable.contains(&card),
        "not castable on my own turn: {legal:?}"
    );
}
