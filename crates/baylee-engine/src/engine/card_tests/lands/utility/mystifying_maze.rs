//! `cards/lands/utility/mystifying_maze.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystifying Maze prints `{T}: Add {C}` and a `{4}, {T}` line that exiles an
/// attacking creature and hands it back at the next end step; the card file
/// leaves the second sentence off entirely (`Coverage::Partial`), so the tap
/// is the whole card and is what is played here — as a real land drop, since
/// `starting_battlefield` places a permanent with `Cause::Setup` and a card
/// nobody watches enter is not the card.
///
/// The reading is the point. The Maze is a nonbasic with no basic land type,
/// so its printed mana ability is an ordinary `(source, index)` entry in
/// `LegalActions::abilities` and never the CR 305.6 shortcut in
/// `mana_abilities` — the half of #159 that a tap-everything helper written
/// against basics alone used to float nothing for. And the colourless mana is
/// read off the pool from an empty one, so nothing else on this board could
/// have produced it.
#[test]
fn mystifying_maze_taps_for_the_colorless_mana_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[mystifying_maze()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    play_land(&mut engine, p0, mystifying_maze());
    let maze =
        on_battlefield(&engine, p0, mystifying_maze()).expect("the Maze landed off the drop");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a land drop leaves a quiet priority: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and the seat that played it holds it");
    assert!(
        legal.abilities.contains(&(maze, 0)),
        "the Maze's only written line is a printed mana ability: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&maze),
        "no basic land type, so CR 305.6 has nothing to name here: {:?}",
        legal.mana_abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before the tap"
    );

    activate(&mut engine, p0, mystifying_maze(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}, and nothing was named to make it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, maze), "the Maze paid its own {{T}}");
}

/// "{4}, {T}: Exile target attacking creature an opponent controls." Out of
/// the opponent's creatures only the one that attacks is on the menu, and in
/// the controller's own attack their attacker is no target at all.
#[test]
fn mystifying_maze_targets_only_an_opponents_attacking_creature() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let mut e = kit::game(
        958,
        &[mystifying_maze(), index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS, index::SERRA_ANGEL],
        &[],
        &[],
    );
    let (mine, their_attacker) = (
        kit::obj(&e, kit::P0, index::GRIZZLY_BEARS),
        kit::obj(&e, kit::P1, index::GRIZZLY_BEARS),
    );
    let their_idler = kit::obj(&e, kit::P1, index::SERRA_ANGEL);

    // The Maze's controller attacks: the opponent's creatures stand idle and
    // their own attacker is not one an opponent controls.
    assert!(kit::attackers_offered(&mut e).contains(&mine));
    e.apply(
        kit::P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(mine, Defender::Player(kit::P1))],
        },
    )
    .unwrap();
    assert!(matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0));
    kit::float(&mut e, kit::P0, &[(ManaColor::Colorless, 4)]);
    let maze = kit::obj(&e, kit::P0, mystifying_maze());
    assert!(
        !kit::offered(&e, mystifying_maze(), kit::P0, 1),
        "neither my own attacker nor the idle creatures are targets"
    );
    assert_p0_refused(&mut e, maze, 1);

    // The opponent attacks with one of their two creatures.
    let _ = kit::attackers_offered(&mut e);
    let blocks = kit::declare(&mut e, &[their_attacker]);
    assert!(blocks.iter().all(|b| b.blocker != their_attacker));
    kit::block(&mut e, &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0),
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Colorless, 4)]);
    activate(&mut e, kit::P0, mystifying_maze(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("a target choice was expected, got {:?}", e.pending());
    };
    assert_eq!(options, vec![their_attacker], "the attacker, nothing else");
    assert!(!options.contains(&their_idler) && !options.contains(&mine));
    aim_at(&mut e, kit::P0, their_attacker);
    kit::settle(&mut e);
    assert_eq!(kit::zone_of(&e, their_attacker), Zone::Exile);
    assert_eq!(kit::zone_of(&e, their_idler), Zone::Battlefield);
    assert_eq!(kit::zone_of(&e, mine), Zone::Battlefield);
}
