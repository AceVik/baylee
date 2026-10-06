//! `cards/sorceries/mv_1/blaze.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blaze is `{X}{R}` for "Blaze deals X damage to any target", and the amount
/// is the whole card: the same sorcery aimed at a seat for X of 3 must read
/// three life off one total, and aimed at a creature for X of 1 must read a
/// dead 1/1 off the board — one cast can prove only one of those, so the hand
/// carries two copies. X is announced as the spell is cast (CR 601.2b) and the
/// target right after it (CR 601.2c), and the range the engine offers for X is
/// bounded by mana that is already *floating*, which is why the Mountains are
/// tapped before any claim is made. The Elf is offered as an option on the
/// first cast and is left standing, so "never named" is a control rather than
/// an absence.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn blaze_deals_its_announced_x_to_the_target_it_names_and_to_no_other() {
    /// Answers one Blaze: the announced X, then the target — `at` when
    /// `creature` is `None`, that creature when it is not. Hands back both
    /// option lists the target question published, so the caller can read what
    /// "any target" meant on this board.
    fn cast_blaze(
        engine: &mut Engine<RegistryLookup>,
        seat: PlayerId,
        x: u32,
        at: PlayerId,
        creature: Option<ObjectId>,
    ) -> (Vec<ObjectId>, Vec<PlayerId>) {
        cast_with_floating(engine, seat, blaze());
        let mut announced = false;
        let mut offered: Option<(Vec<ObjectId>, Vec<PlayerId>)> = None;
        for _ in 0..12 {
            if announced && offered.is_some() {
                break;
            }
            match engine.pending().clone() {
                Pending::ChooseNumber {
                    player, min, max, ..
                } => {
                    assert!(
                        (min..=max).contains(&x),
                        "X of {x} is inside the range the cast offered: {min}..={max}"
                    );
                    engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
                    announced = true;
                }
                Pending::ChooseTargets {
                    player,
                    options,
                    player_options,
                    ..
                } => {
                    let objects: Vec<ObjectId> = creature.into_iter().collect();
                    let players: Vec<PlayerId> = if creature.is_some() {
                        Vec::new()
                    } else {
                        vec![at]
                    };
                    engine
                        .apply(player, PlayerAction::ChooseTargets { objects, players })
                        .unwrap();
                    offered = Some((options, player_options));
                }
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected while casting Blaze: {other:?}"),
            }
        }
        assert!(
            announced && offered.is_some(),
            "the cast asked for its X and its target, then {:?}",
            engine.pending()
        );
        offered.expect("checked above")
    }

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[blaze(), blaze()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves =
        on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf stands across the table");

    // Both Mountains counts are read off the *pool*, because that is where the
    // engine reads what an `{X}{R}` can pay: six for the first cast, and the
    // two left over for the second.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains tapped, so the cast has four mana to announce an X of 3 with"
    );

    let (objects, players) = cast_blaze(&mut engine, p0, 3, p1, None);
    assert!(
        players.contains(&p0) && players.contains(&p1),
        "CR 115.4: \"any target\" counts both seats in the same choice: {players:?}"
    );
    assert!(
        objects.contains(&elves),
        "and the creature across the table is one of the object options: {objects:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[1].life,
        17,
        "X of 3 is three damage to the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the seat that aimed it is untouched"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf was offered as an option and never named, so it still stands"
    );
    assert!(
        in_graveyard(&engine, p0, blaze()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{X}}{{R}} with X of 3 is four of the six Mountains"
    );

    let (objects, players) = cast_blaze(&mut engine, p0, 1, p1, Some(elves));
    assert!(
        objects.contains(&elves),
        "the same creature is still on the menu, which is the answer this cast takes: {objects:?}"
    );
    assert!(
        players.contains(&p0) && players.contains(&p1),
        "and the two seats are still there, as the answer it declines: {players:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "X of 1 on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "\"any target\" aimed at a creature does not touch the seat behind it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{X}}{{R}} with X of 1 spent the last two Mountains"
    );
}
