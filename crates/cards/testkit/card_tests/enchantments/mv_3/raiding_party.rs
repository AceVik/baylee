//! `cards/enchantments/mv_3/raiding_party.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn party_card() -> CardIndex {
    card_index("1d9b8859-14e3-4bbe-ad92-916b25a37b31")
}
fn orcs_card() -> CardIndex {
    // Ironclaw Orcs, an Orc.
    card_index("c645a616-0d7d-416c-b5f3-057b3a1666a0")
}
fn infantry_card() -> CardIndex {
    // Icatian Infantry, a white creature.
    card_index("9eb274f3-ef55-4545-81b4-29454a48f075")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}
fn disenchant_card() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}
fn naturalize_card() -> CardIndex {
    card_index("bdb3ca68-ec1f-4e16-81cc-d23f8f52c728")
}

fn zone_of(e: &Engine<RegistryLookup>, id: ObjectId) -> Option<Zone> {
    e.state().object(id).map(|o| o.zone)
}

/// Seat 0: Raiding Party, an Orc and `my_plains` Plains (and `my_white`
/// white creatures); seat 1: `their_plains` Plains and `their_white` white
/// creatures. Stops at p0's first main phase.
fn game(
    my_plains: usize,
    my_white: usize,
    their_plains: usize,
    their_white: usize,
) -> Engine<RegistryLookup> {
    let mut mine = vec![party_card(), orcs_card()];
    mine.extend(std::iter::repeat_n(plains(), my_plains));
    mine.extend(std::iter::repeat_n(infantry_card(), my_white));
    let mut theirs: Vec<CardIndex> = std::iter::repeat_n(plains(), their_plains).collect();
    theirs.extend(std::iter::repeat_n(infantry_card(), their_white));
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &mine)
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

/// Presses the sacrifice ability, paying with the Orc, then answers every
/// question that follows: a seat's creature question is answered with all of
/// its white creatures (when `tap` is set), a Plains question with the first
/// `keep` Plains of `picks` for that seat.
#[track_caller]
fn run(
    e: &mut Engine<RegistryLookup>,
    tap: bool,
    picks: &[(PlayerId, Vec<ObjectId>)],
    all_plains: &[ObjectId],
) {
    activate(e, P0, party_card(), 1);
    let orc = on_battlefield(e, P0, orcs_card()).expect("the Orc");
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("the Orc is a sacrifice choice, got {:?}", e.pending())
    };
    assert!(options.contains(&orc));
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![orc] })
        .expect("the Orc is a legal sacrifice");
    for _ in 0..12 {
        match e.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                max,
                ..
            } => {
                let is_plains_question = options.iter().all(|o| all_plains.contains(o));
                let answer: Vec<ObjectId> = if is_plains_question {
                    picks
                        .iter()
                        .filter(|(seat, _)| *seat == player)
                        .flat_map(|(_, v)| v.iter().copied())
                        .filter(|o| options.contains(o))
                        .take(usize::from(max))
                        .collect()
                } else if tap {
                    options.clone()
                } else {
                    vec![]
                };
                e.apply(player, PlayerAction::ChooseObjects { objects: answer })
                    .expect("the answer came out of the question");
            }
            Pending::Priority { .. } if stack_is_empty(e) => return,
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question {other:?}"),
        }
    }
    panic!("still asking: {:?}", e.pending());
}

/// Raiding Party (1d9b8859-14e3-4bbe-ad92-916b25a37b31) — "Sacrifice an Orc:
/// Each player may tap any number of untapped white creatures they control.
/// For each creature tapped this way, that player chooses up to two Plains.
/// Then destroy all Plains that weren't chosen this way by any player." One
/// white creature tapped by p1 spares two Plains; everything else dies.
#[test]
fn raiding_party_spares_two_plains_per_white_creature_tapped() {
    let mut e = game(1, 0, 3, 1);
    let mine = all_on_battlefield(&e, P0, plains());
    let theirs = all_on_battlefield(&e, P1, plains());
    let infantry = on_battlefield(&e, P1, infantry_card()).expect("their white creature");
    let orc = on_battlefield(&e, P0, orcs_card()).expect("the Orc");
    let all: Vec<ObjectId> = mine.iter().chain(&theirs).copied().collect();

    run(
        &mut e,
        true,
        &[(P1, vec![theirs[0], theirs[1], theirs[2]])],
        &all,
    );

    assert_eq!(
        zone_of(&e, orc),
        Some(Zone::Graveyard),
        "the Orc was the cost"
    );
    assert!(
        e.state()
            .object(infantry)
            .expect("still out")
            .status
            .contains(Status::TAPPED),
        "the white creature was tapped"
    );
    assert_eq!(zone_of(&e, theirs[0]), Some(Zone::Battlefield), "chosen");
    assert_eq!(zone_of(&e, theirs[1]), Some(Zone::Battlefield), "chosen");
    assert_eq!(
        zone_of(&e, theirs[2]),
        Some(Zone::Graveyard),
        "one creature tapped is two Plains chosen, no more"
    );
    assert_eq!(
        zone_of(&e, mine[0]),
        Some(Zone::Graveyard),
        "p0 tapped nothing and chose nothing: its Plains are destroyed"
    );
}

/// Raiding Party — "may tap": if nobody taps a white creature, every Plains
/// is destroyed.
#[test]
fn raiding_party_destroys_every_plains_when_nobody_taps_anything() {
    let mut e = game(2, 0, 2, 1);
    let mine = all_on_battlefield(&e, P0, plains());
    let theirs = all_on_battlefield(&e, P1, plains());
    let infantry = on_battlefield(&e, P1, infantry_card()).expect("their white creature");
    let all: Vec<ObjectId> = mine.iter().chain(&theirs).copied().collect();

    run(&mut e, false, &[], &all);

    for p in &all {
        assert_eq!(
            zone_of(&e, *p),
            Some(Zone::Graveyard),
            "no Plains was chosen"
        );
    }
    assert!(
        !e.state()
            .object(infantry)
            .expect("still out")
            .status
            .contains(Status::TAPPED),
        "nothing was tapped"
    );
}

/// Raiding Party — "Each player": both players tap a white creature, each
/// chooses its own two, and the Plains nobody chose are destroyed.
#[test]
fn raiding_party_lets_each_player_choose_for_its_own_creatures() {
    let mut e = game(3, 1, 3, 1);
    let mine = all_on_battlefield(&e, P0, plains());
    let theirs = all_on_battlefield(&e, P1, plains());
    let all: Vec<ObjectId> = mine.iter().chain(&theirs).copied().collect();

    run(
        &mut e,
        true,
        &[
            (P0, vec![mine[0], mine[1]]),
            (P1, vec![theirs[1], theirs[2]]),
        ],
        &all,
    );

    assert_eq!(zone_of(&e, mine[0]), Some(Zone::Battlefield));
    assert_eq!(zone_of(&e, mine[1]), Some(Zone::Battlefield));
    assert_eq!(
        zone_of(&e, mine[2]),
        Some(Zone::Graveyard),
        "p0 left one out"
    );
    assert_eq!(
        zone_of(&e, theirs[0]),
        Some(Zone::Graveyard),
        "p1 left one out"
    );
    assert_eq!(zone_of(&e, theirs[1]), Some(Zone::Battlefield));
    assert_eq!(zone_of(&e, theirs[2]), Some(Zone::Battlefield));
}

/// Raiding Party — "Sacrifice an Orc": a creature that is not an Orc cannot
/// pay.
#[test]
fn raiding_party_cannot_be_paid_with_a_non_orc() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[party_card(), bears_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let party = on_battlefield(&e, P0, party_card()).expect("Raiding Party");
    assert!(!priority_offer(&e).abilities.contains(&(party, 1)));
}

/// Raiding Party — "This enchantment can't be the target of white spells or
/// abilities from white sources." A white Disenchant cannot name it, a green
/// Naturalize can.
#[test]
fn raiding_party_cannot_be_targeted_by_white_spells_but_can_by_others() {
    // White: Disenchant, with Sol Ring as the one legal target.
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[party_card(), sol_ring()])
        .hand(1, &[disenchant_card()])
        .battlefield(1, &[plains(), plains()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, P1);
    let party = on_battlefield(&e, P0, party_card()).expect("Raiding Party");
    let ring = on_battlefield(&e, P0, sol_ring()).expect("Sol Ring");
    cast_from_hand(&mut e, P1, disenchant_card());
    let menu = pass_until_targets(&mut e, P1);
    assert!(menu.contains(&ring), "an ordinary artifact is a target");
    assert!(
        !menu.contains(&party),
        "a white spell may not target Raiding Party"
    );

    // Green: Naturalize may.
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[party_card(), sol_ring()])
        .hand(1, &[naturalize_card()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, P1);
    let party = on_battlefield(&e, P0, party_card()).expect("Raiding Party");
    cast_from_hand(&mut e, P1, naturalize_card());
    let menu = pass_until_targets(&mut e, P1);
    assert!(menu.contains(&party), "a green spell may target it");
}
