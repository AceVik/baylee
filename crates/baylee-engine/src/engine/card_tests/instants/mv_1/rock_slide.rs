//! `cards/instants/mv_1/rock_slide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rock_slide() -> CardIndex {
    card_index("0b2e7c1a-dc07-4138-a560-dc80fa723860")
}

struct Combat {
    engine: Engine<RegistryLookup>,
    /// P1's two attacking Grizzly Bears.
    bears: Vec<ObjectId>,
    /// P1's attacking Serra Angel (a flier).
    angel: ObjectId,
    /// P1's Llanowar Elves, which stays home.
    idle_elves: ObjectId,
    /// P0's Llanowar Elves, which blocks the first Bears when `block` is set.
    blocker: ObjectId,
}

/// It is P1's turn: two Bears and a Serra Angel attack P0; P1's Elves stays
/// home. P0's Elves blocks the first Bears if `block`. The game stops with P0
/// holding priority and five Mountains.
fn combat(block: bool) -> Combat {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(
            1,
            &[
                GRIZZLY_BEARS,
                GRIZZLY_BEARS,
                serra_angel(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[rock_slide()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let bears: Vec<_> = all_on_battlefield(&engine, P1, GRIZZLY_BEARS);
    assert_eq!(bears.len(), 2);
    let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
    let idle_elves = on_battlefield(&engine, P1, llanowar_elves()).unwrap();
    let blocker = on_battlefield(&engine, P0, llanowar_elves()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears[0], Defender::Player(P0)),
                    (bears[1], Defender::Player(P0)),
                    (angel, Defender::Player(P0)),
                ],
            },
        )
        .unwrap();
    if block {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        engine
            .apply(
                P0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, bears[0])],
                },
            )
            .unwrap();
    }
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    Combat {
        engine,
        bears,
        angel,
        idle_elves,
        blocker,
    }
}

/// Casts Rock Slide for `x` and returns the target menu it offers.
fn announce(engine: &mut Engine<RegistryLookup>, x: u32) -> Vec<ObjectId> {
    tap_all_mana(engine, P0);
    cast_with_floating(engine, P0, rock_slide());
    engine.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the target choice, got {:?}", engine.pending());
    };
    options
}

/// `Rock Slide` ({X}{R}, Instant, `Coverage::Implemented`):
/// `Rock Slide deals X damage divided as you choose among any number of target
/// attacking or blocking creatures without flying.` The menu holds the
/// attackers and the blocker without flying, and not the Angel (flying) or the
/// Elves that stays home; X = 4 divided 2 and 2 kills both Bears.
#[test]
fn rock_slide_deals_x_damage_divided_among_attacking_creatures_without_flying() {
    let Combat {
        mut engine,
        bears,
        angel,
        idle_elves,
        blocker,
    } = combat(true);

    let options = announce(&mut engine, 4);
    assert!(options.contains(&bears[0]) && options.contains(&bears[1]));
    assert!(
        options.contains(&blocker),
        "a blocking creature is a target too"
    );
    assert!(!options.contains(&angel), "a flying attacker is not");
    assert!(
        !options.contains(&idle_elves),
        "a creature not in combat is not"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: bears.clone(),
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bears[0]).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(bears[1]).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(angel).unwrap().damage, 0);
}

/// "Divided as you choose": 3 and 1 kills the first Bears and leaves the
/// second with 1 damage.
#[test]
fn rock_slide_divides_the_damage_as_chosen() {
    let Combat {
        mut engine, bears, ..
    } = combat(false);

    announce(&mut engine, 4);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: bears.clone(),
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bears[0]).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(bears[1]).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(bears[1]).unwrap().damage, 1);
}
