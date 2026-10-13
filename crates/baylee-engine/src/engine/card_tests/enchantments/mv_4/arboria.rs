//! `cards/enchantments/mv_4/arboria.rs`, played.
//!
//! Arboria: "Creatures can't attack a player unless that player cast a spell
//! or put a nontoken permanent onto the battlefield during their last turn."
//!
//! What a seat is *allowed* to do is read where the engine says it: the
//! declare-attackers question (`Pending::ChooseAttackers`) lists the creatures
//! that may attack and, for one that may attack only some of the defenders,
//! which (CR 508.1). A creature every restriction shuts out is not listed at
//! all. Each negative below has its positive twin (the same board with the
//! player having done what the sentence asks), because an attack that is
//! missing from the menu proves nothing on a board where it never could have
//! been there.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn arboria() -> CardIndex {
    card_index("acb3e93c-a1d3-458f-b8c3-c426cd359fa4")
}

/// `{2}, {T}`: create a 0/1 colourless Pest artifact creature token. A token
/// maker that is a permanent rather than a spell, so a turn can put a token
/// onto the battlefield without casting anything.
fn nuisance_engine() -> CardIndex {
    card_index("288afdb9-708d-4a52-991a-e1b07f62ee99")
}

/// Walks to `seat`'s declaration of attackers (declaring nothing on the way
/// for any other seat) and says whether `creature` is offered against
/// `defender`: listed as an attacker, and not held by a limit to a different
/// part of the defenders (CR 508.1c).
///
/// It stops at the question, so a caller that wants to attack answers it.
#[track_caller]
fn offered_against(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    creature: ObjectId,
    defender: Defender,
) -> bool {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    let Pending::ChooseAttackers {
        attackers, limits, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the declaration")
    };
    attackers.contains(&creature)
        && limits
            .iter()
            .find(|limit| limit.creature == creature)
            .is_none_or(|limit| limit.defenders.contains(&defender))
}

/// Declares `creature` attacking `defender` at the question in front of the
/// engine, and says whether the engine took the declaration.
#[track_caller]
fn declare(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    creature: ObjectId,
    defender: Defender,
) -> bool {
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(creature, defender)],
            },
        )
        .is_ok()
}

/// A duel on player 0's first main phase: player 0 holds Arboria (unless
/// `without` says not) and a Gray Ogre, player 1 holds `theirs` on the
/// battlefield and `hand` in hand. Returns the engine and the Ogre.
#[track_caller]
fn the_table(
    with_arboria: bool,
    theirs: &[CardIndex],
    hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let mine: Vec<CardIndex> = if with_arboria {
        vec![arboria(), gray_ogre()]
    } else {
        vec![gray_ogre()]
    };
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, theirs)
        .hand(1, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let ogre = on_battlefield(&engine, kit::P0, gray_ogre()).expect("the Ogre");
    (engine, ogre)
}

/// "…unless that player cast a spell or put a nontoken permanent onto the
/// battlefield during their last turn": a player who has not had a turn yet
/// has not done either, so on the first turn of the game they cannot be
/// attacked. The same board without Arboria is the positive control: the
/// Ogre is offered, so its absence below is Arboria's doing and nobody's
/// summoning sickness.
#[test]
fn arboria_a_player_who_has_not_had_a_turn_yet_cannot_be_attacked() {
    let defender = Defender::Player(kit::P1);

    let (mut control, ogre) = the_table(false, &[], &[]);
    assert!(
        offered_against(&mut control, kit::P0, ogre, defender),
        "without Arboria the Ogre may attack player 1 on turn one"
    );
    assert!(declare(&mut control, kit::P0, ogre, defender));

    let (mut engine, ogre) = the_table(true, &[], &[]);
    assert_eq!(engine.state().turn.number, 1, "the first turn of the game");
    assert!(
        !offered_against(&mut engine, kit::P0, ogre, defender),
        "player 1 has had no turn, so cast and put nothing during it"
    );
    assert!(
        !declare(&mut engine, kit::P0, ogre, defender),
        "and the engine refuses the declaration, not only the menu"
    );
}

/// A player who spent their last turn doing nothing is still shut out on the
/// turns after the first: player 1 passes turn two without casting a spell or
/// putting anything onto the battlefield, and on turn three the Ogre is not
/// offered against them.
#[test]
fn arboria_a_player_who_did_nothing_during_their_last_turn_cannot_be_attacked() {
    let defender = Defender::Player(kit::P1);
    let (mut engine, ogre) = the_table(true, &[], &[]);

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    assert_eq!(engine.state().turn.number, 3, "player 1's turn two went by");
    assert!(
        !offered_against(&mut engine, kit::P0, ogre, defender),
        "player 1 passed their whole turn"
    );
    assert!(!declare(&mut engine, kit::P0, ogre, defender));
}

/// "…put a nontoken permanent onto the battlefield during their last turn":
/// player 1 plays a land on their turn (a permanent put onto the battlefield
/// with no spell cast), and on the next turn the Ogre may attack them. Twin
/// of the idle test above, on the same board.
#[test]
fn arboria_a_player_who_played_a_land_during_their_last_turn_can_be_attacked() {
    let defender = Defender::Player(kit::P1);
    let (mut engine, ogre) = the_table(true, &[], &[forest()]);

    reach_their_main_phase(&mut engine, kit::P1);
    play_land(&mut engine, kit::P1, forest());
    kit::walk_to_next_own_main(&mut engine, kit::P0);
    assert!(
        offered_against(&mut engine, kit::P0, ogre, defender),
        "they put a nontoken permanent onto the battlefield during turn two"
    );
    assert!(
        declare(&mut engine, kit::P0, ogre, defender),
        "and the engine takes the attack"
    );
}

/// "…cast a spell…": player 1 casts a spell and nothing else on their turn —
/// a creature spell that player 0 counters, so that no permanent ever
/// arrives — and can be attacked. Ruling (2022-12-08): the effect cares about
/// the actions taken by players, not their results; a countered spell still
/// counts.
#[test]
fn arboria_a_spell_that_was_countered_still_counts_as_cast() {
    let defender = Defender::Player(kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arboria(), gray_ogre(), island(), island()])
        .hand(0, &[counterspell()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let ogre = on_battlefield(&engine, kit::P0, gray_ogre()).expect("the Ogre");

    reach_their_main_phase(&mut engine, kit::P1);
    cast_from_hand(&mut engine, kit::P1, llanowar_elves());
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    engine
        .apply(kit::P1, PlayerAction::PassPriority)
        .expect("player 1 passes with the spell on the stack");
    tap_all_mana(&mut engine, kit::P0);
    cast_with_floating(&mut engine, kit::P0, counterspell());
    aim_at(&mut engine, kit::P0, elves);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, kit::P1, llanowar_elves()).is_some(),
        "the Elves were countered"
    );
    assert!(
        on_battlefield(&engine, kit::P1, llanowar_elves()).is_none(),
        "so no permanent entered: only the cast can have counted"
    );

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    assert!(
        offered_against(&mut engine, kit::P0, ogre, defender),
        "a spell was cast during their last turn, countered or not"
    );
}

/// "…a **nontoken** permanent…": a turn in which a player's only new
/// permanent is a token made by an ability, with no spell cast, does not
/// open them to attack. Player 1 activates a Nuisance Engine for a Pest
/// token on their turn; on turn three the Ogre is still not offered. The
/// land test above is the twin: the same turn with a land played instead.
#[test]
fn arboria_a_token_is_not_a_nontoken_permanent() {
    let defender = Defender::Player(kit::P1);
    let (mut engine, ogre) = the_table(true, &[nuisance_engine(), mountain(), mountain()], &[]);

    reach_their_main_phase(&mut engine, kit::P1);
    tap_all_mana(&mut engine, kit::P1);
    activate(&mut engine, kit::P1, nuisance_engine(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        tokens_of(&engine, kit::P1).len(),
        1,
        "the Engine made its Pest on their turn"
    );

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    assert!(
        !offered_against(&mut engine, kit::P0, ogre, defender),
        "a token is not a nontoken permanent, and no spell was cast"
    );
}

/// "…during **their** last turn": what player 1 does on player 0's turn is
/// not what the sentence reads. Player 1 passes turn two doing nothing and
/// then casts a Lightning Bolt in the middle of turn three, before the
/// attack; the Ogre is still not offered against them. (The spell they cast
/// on turn two, in the test above, is what opens them up.)
#[test]
fn arboria_a_spell_cast_on_the_attackers_own_turn_does_not_count() {
    let defender = Defender::Player(kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arboria(), gray_ogre()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let ogre = on_battlefield(&engine, kit::P0, gray_ogre()).expect("the Ogre");

    // Player 1 idles through turn two; on turn three player 0 passes in the
    // first main phase and player 1 gets priority.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == kit::P0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P1)
    });
    cast_from_hand(&mut engine, kit::P1, lightning_bolt());
    engine
        .apply(
            kit::P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![kit::P0],
            },
        )
        .expect("a player is a legal target for the Bolt");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, kit::P1, lightning_bolt()).is_some(),
        "the Bolt was cast and resolved"
    );

    assert!(
        !offered_against(&mut engine, kit::P0, ogre, defender),
        "it was cast on turn three, not during their last turn"
    );
}

/// "Creatures can't attack a player…": every creature, whoever controls it.
/// Arboria is on player 0's side, and player 1's Ogre is turned away from
/// player 0 on turn two when player 0 did nothing on turn one. Twin: with a
/// land played on turn one, the same Ogre may attack.
#[test]
fn arboria_binds_the_creatures_of_every_player_and_protects_its_own_controller() {
    for (plays_a_land, may_attack) in [(false, false), (true, true)] {
        let defender = Defender::Player(kit::P0);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[arboria()])
            .hand(0, &[forest()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, kit::P0);
        if plays_a_land {
            play_land(&mut engine, kit::P0, forest());
        }
        let ogre = on_battlefield(&engine, kit::P1, gray_ogre()).expect("player 1's Ogre");

        assert_eq!(
            offered_against(&mut engine, kit::P1, ogre, defender),
            may_attack,
            "player 0 {} a land on turn one",
            if plays_a_land {
                "played"
            } else {
                "did not play"
            }
        );
        assert_eq!(engine.state().turn.number, 2, "player 1's own turn");
    }
}

/// Ruling (2022-12-08): "Arboria doesn't stop creatures from attacking
/// planeswalkers, regardless of what their controllers did or didn't do
/// during their last turns." Player 1, who has had no turn, holds a Jace: the
/// Ogre is offered against the planeswalker and only against it.
#[test]
fn arboria_never_protects_a_planeswalker() {
    let (mut engine, ogre) = the_table(true, &[jace_the_mind_sculptor()], &[]);
    let jace = on_battlefield(&engine, kit::P1, jace_the_mind_sculptor()).expect("Jace");
    kit::set_loyalty(&mut engine, kit::P1, jace, 3);
    assert!(
        is_on_the_table(&engine, jace),
        "Jace is standing, so it can be attacked"
    );

    assert!(
        !offered_against(&mut engine, kit::P0, ogre, Defender::Player(kit::P1)),
        "the player behind it is protected"
    );
    assert!(
        offered_against(&mut engine, kit::P0, ogre, Defender::Planeswalker(jace)),
        "the planeswalker is not"
    );
    assert!(
        declare(&mut engine, kit::P0, ogre, Defender::Planeswalker(jace)),
        "and the engine takes the attack on it"
    );
}

fn is_on_the_table(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    kit::is_on_battlefield(engine, id)
}

/// The sentence is read per player. Three seats: player 1 does nothing on
/// turn two and player 2 plays a land on turn three; on turn four player 0's
/// Ogre is offered against player 2 and not against player 1.
#[test]
fn arboria_is_read_for_each_defending_player_alone() {
    let (p1, p2) = (kit::P1, kit::P2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[arboria(), gray_ogre()])
        .hand(2, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let ogre = on_battlefield(&engine, kit::P0, gray_ogre()).expect("the Ogre");

    reach_their_main_phase(&mut engine, p2);
    assert_eq!(engine.state().turn.number, 3, "player 2's turn");
    play_land(&mut engine, p2, forest());

    assert!(
        offered_against(&mut engine, kit::P0, ogre, Defender::Player(p2)),
        "player 2 played a land on their last turn"
    );
    assert_eq!(engine.state().turn.number, 4, "player 0's second turn");
    assert!(
        !offered_against(&mut engine, kit::P0, ogre, Defender::Player(p1)),
        "player 1 did nothing on theirs"
    );
}
