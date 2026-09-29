//! Effects where **each player chooses among their own permanents**.
//!
//! Three effects share one shape: the effect names a filter and a set of
//! players, and every one of those players is asked, in turn, to pick one of
//! their own permanents matching it. What happens to the pick is the only
//! difference — [`Effect::SacrificeFilter`] sends it to the graveyard,
//! [`Effect::DestroyChosenForPlayers`] destroys it.
//!
//! What matters about the shape, and what these tests exist to pin, is that
//! **the choice belongs to the permanent's controller and not to the
//! spell's**. An edict that let the caster pick would be a targeted removal
//! spell wearing an edict's text, and nothing in the type system says
//! otherwise: the chain hands `Pending::ChooseCards` a `player`, and a wrong
//! one compiles.
//!
//! The options are also a *filter over that player's own battlefield*, so a
//! mode naming nontoken creatures must not offer a token — which is the
//! second thing a wrong implementation gets away with silently, because a
//! board with one creature on it agrees with every filter.

use super::testkit::{
    Duel, RegistryLookup, SEED, card_index, cast_from_hand, keep_mulligans, quiet_creature,
    reach_main_phase, stack_is_empty,
};
use super::*;
use baylee_core::ids::CardIndex;

fn sheoldreds_edict() -> CardIndex {
    card_index("217062f5-96f1-454c-9507-17f34ef37070")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// Casts the edict from seat 0 and answers the mode question with `mode`.
fn cast_edict(mode: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[sheoldreds_edict()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    reach_main_phase(&mut engine, seat);
    cast_from_hand(&mut engine, seat, sheoldreds_edict());

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "a modal spell asks its mode on the way to the stack (CR 601.2b), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 3, "the edict prints three modes");
    engine
        .apply(seat, PlayerAction::ChooseMode(mode))
        .expect("naming a printed mode is legal");
    engine
}

#[test]
fn an_edict_is_answered_by_the_creature_s_controller_and_not_the_caster() {
    let mut engine = cast_edict(0);
    let them = PlayerId::new(1);

    // Pass priority until the spell resolves and the question is asked.
    let seat = PlayerId::new(0);
    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let asked = match engine.pending() {
            Pending::Priority { player, .. } => *player,
            other => panic!("expected priority or the edict's question, got {other:?}"),
        };
        engine
            .apply(asked, PlayerAction::PassPriority)
            .expect("passing is always legal");
        let _ = seat;
    }

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the edict asks somebody to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, them,
        "an edict is not removal: the opponent picks, the caster does not"
    );
    assert_eq!(options.len(), 2, "both of their creatures may be given up");
    assert_eq!(
        (min, max),
        (1, 1),
        "'sacrifices a creature' is not optional"
    );

    let victim = options[0];
    let survivor = options[1];
    engine
        .apply(
            them,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("naming one of the offered creatures is legal");

    assert_eq!(
        engine.state().object(victim).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "the chosen creature was not sacrificed"
    );
    assert_eq!(
        engine.state().object(survivor).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "an edict takes one creature, not the board"
    );
}

#[test]
fn the_token_mode_offers_no_card_and_therefore_asks_nobody() {
    // Mode two is "a creature token of their choice", and seat one's two
    // creatures are cards. A filter that matched anyway would be invisible on
    // a board where every creature is legal, which is why the discriminating
    // board has *only* the wrong kind on it.
    let mut engine = cast_edict(1);
    let seat = PlayerId::new(0);
    let them = PlayerId::new(1);

    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => panic!(
                "no token is on the battlefield, yet {} permanent(s) were offered",
                options.len()
            ),
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is legal");
            }
            other => panic!("unexpected question: {other:?}"),
        }
        if stack_is_empty(&engine) {
            break;
        }
    }

    assert!(stack_is_empty(&engine), "the edict never left the stack");
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(them))
            .is_empty(),
        "nothing of theirs was sacrificed, so their graveyard stays empty"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(seat))
            .len(),
        1,
        "the edict itself goes to its own owner's graveyard, and only it"
    );
}

fn sheoldred() -> CardIndex {
    card_index("97652492-7906-4d79-983c-fa1dc1239eba")
}

fn taoist_hermit() -> CardIndex {
    card_index("960b1251-12af-4766-8df1-bb98e7e967f5")
}

/// Chapter I of The True Scriptures: "For each opponent, destroy up to one
/// target creature or planeswalker that player controls."
///
/// The road to it is part of the test: Sheoldred is a creature until
/// `{4}{B}` exiles her and returns her transformed, and that ability is
/// offered only while an opponent holds eight cards in their graveyard.
///
/// Three seats, because a duel cannot tell "one per opponent" from "one":
/// the Saga's controller is asked once for each opponent, each question
/// offers only that player's permanents, a hexproof creature is not on the
/// menu (it is targeting), and both answers die together.
#[test]
#[allow(clippy::too_many_lines)] // the road to the back face is part of the test
fn a_saga_chapter_targets_one_permanent_of_each_opponent() {
    let mut engine = Duel::table(SEED, swamp(), 3)
        .battlefield(
            0,
            &[sheoldred(), swamp(), swamp(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[quiet_creature(), quiet_creature(), taoist_hermit()])
        .battlefield(2, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    let (left, right) = (PlayerId::new(1), PlayerId::new(2));
    super::testkit::seed_graveyard(&mut engine, left, 8);
    reach_main_phase(&mut engine, seat);

    // Tap for {4}{B} and press the transform ability.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("a swamp taps for black");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after tapping, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .next()
        .expect("eight cards in their graveyard make the transform legal");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the transform ability activates at sorcery speed");

    let controller_of =
        |e: &Engine<RegistryLookup>, id: ObjectId| e.state().object(id).map(|o| o.controller);
    let mut asked = Vec::new();
    let mut doomed = Vec::new();
    for _ in 0..80 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, seat, "the Saga's controller chooses the targets");
                assert_eq!((min, max), (0, 1), "up to one per opponent");
                let owners: Vec<_> = options.iter().map(|o| controller_of(&engine, *o)).collect();
                assert!(
                    owners.windows(2).all(|w| w[0] == w[1]),
                    "one question offers one opponent's permanents: {owners:?}"
                );
                asked.push(owners[0].expect("a permanent"));
                let pick = options[0];
                doomed.push(pick);
                if owners[0] == Some(left) {
                    assert_eq!(
                        options.len(),
                        2,
                        "the two Elves and not the hexproof Hermit"
                    );
                }
                engine
                    .apply(
                        seat,
                        PlayerAction::ChooseTargets {
                            objects: vec![pick],
                            players: vec![],
                        },
                    )
                    .expect("an offered permanent");
            }
            Pending::Priority { player, .. } => {
                if asked.len() == 2 && stack_is_empty(&engine) {
                    break;
                }
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is legal");
            }
            other => panic!("unexpected question on the way through chapter I: {other:?}"),
        }
    }
    assert_eq!(
        asked,
        vec![left, right],
        "each opponent once, in turn order"
    );
    for id in doomed {
        assert_eq!(
            engine.state().object(id).map(|o| o.zone),
            Some(crate::zone::Zone::Graveyard),
            "a chosen target was not destroyed"
        );
    }
}
