//! What the house brings to its search through forks (`foresight`): the
//! answers it tries, the value of a leaf, the choice among what was tried,
//! and what a fair seat supposes is hidden.

use super::*;
use crate::foresight::prior::public_prior;
use crate::foresight::{Candidates, choose};
use baylee_core::ids::Defender;

fn index(name: &str) -> baylee_core::ids::CardIndex {
    baylee_cards::decks::by_name(name).expect("a card of that name in the pool")
}

/// The level's own answer stands unless another's mean over the same worlds
/// beats it by more than the margin; then the best of those is given.
#[test]
fn the_levels_own_answer_stands_within_the_margin() {
    let own = vec![Some(0.50), Some(0.50)];
    let near = vec![Some(0.51), Some(0.51)];
    let better = vec![Some(0.60), Some(0.56)];
    let best = vec![Some(0.70), Some(0.66)];
    assert_eq!(choose(&[own.clone(), near.clone()], 0.02), 0);
    assert_eq!(choose(&[own.clone(), near, better.clone()], 0.02), 2);
    assert_eq!(choose(&[own.clone(), better, best], 0.02), 2);
    assert_eq!(choose(&[own], 0.02), 0);
    assert_eq!(choose(&[], 0.02), 0);
}

/// An answer the engine refused in a world is compared over the worlds it
/// and the level's own both finished, and one refused everywhere is never
/// given.
#[test]
fn a_refused_answer_is_compared_over_the_worlds_it_has() {
    let own = vec![Some(0.9), Some(0.1)];
    // Better than the own answer in the one world both have.
    let partial = vec![None, Some(0.5)];
    let refused = vec![None, None];
    assert_eq!(choose(&[own.clone(), refused.clone()], 0.02), 0);
    assert_eq!(choose(&[own, refused, partial], 0.02), 2);
}

/// An attack is tried beside all, none, and itself with one creature more
/// or fewer; each is an answer the question takes, and none twice.
#[test]
fn an_attack_is_tried_beside_its_neighbours() {
    let (a, b, c) = (obj(1), obj(2), obj(3));
    let them = Defender::Player(PlayerId::new(1));
    let pending = Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers: vec![a, b, c],
        defenders: vec![them],
        required: vec![],
        alone: vec![],
        minimum_required: None,
        limits: vec![],
    };
    let own = PlayerAction::DeclareAttackers {
        attackers: vec![(a, them), (b, them)],
    };
    let v = view(0, &[20, 20], vec![]);
    let Candidates { answers } = agent().foresight_candidates(&v, &pending, &own, 16);
    let sets: Vec<Vec<ObjectId>> = answers
        .iter()
        .map(|answer| match answer {
            PlayerAction::DeclareAttackers { attackers } => {
                attackers.iter().map(|(id, _)| *id).collect()
            }
            other => panic!("not a declaration: {other:?}"),
        })
        .collect();
    assert_eq!(
        sets,
        vec![vec![a, b], vec![a, b, c], vec![], vec![b], vec![a],]
    );
    // The budget cuts the list, the level's own answer first.
    let few = agent().foresight_candidates(&v, &pending, &own, 2).answers;
    assert_eq!(few.len(), 2);
    assert_eq!(few[0], own);
    // A creature that must attack: the declarations without it are no
    // answers and are not tried.
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = pending
    else {
        unreachable!()
    };
    let bound = Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers,
        defenders,
        required: vec![a],
        alone: vec![],
        minimum_required: None,
        limits: vec![],
    };
    let answers = agent().foresight_candidates(&v, &bound, &own, 16).answers;
    assert!(
        answers
            .iter()
            .all(|answer| bound.answer_fault(answer).is_none())
    );
    assert!(!answers.contains(&PlayerAction::DeclareAttackers { attackers: vec![] }));
}

/// A question a search does not take has the level's own answer alone.
#[test]
fn a_question_outside_the_roots_has_one_candidate() {
    let pending = Pending::Mulligan {
        player: PlayerId::new(0),
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let v = view(0, &[20, 20], vec![]);
    let answers = agent()
        .foresight_candidates(&v, &pending, &PlayerAction::MulliganKeep, 8)
        .answers;
    assert_eq!(answers, vec![PlayerAction::MulliganKeep]);
}

/// The leaf value: even where the table is even but for the turn, higher
/// with more life or a creature more, and settled where a side is gone.
#[test]
fn a_leaf_is_worth_more_with_more_on_its_side() {
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let value =
        |lives: &[i32], board: Vec<PublicObject>| agent().foresight_value(&view(0, lives, board));
    let even = value(&[20, 20], vec![]);
    assert!((0.4..0.6).contains(&even), "{even}");
    assert!(value(&[20, 10], vec![]) > even);
    assert!(value(&[5, 20], vec![]) < even);
    assert!(value(&[20, 20], vec![permanent(obj(1), me, 3)]) > even);
    assert!(value(&[20, 20], vec![permanent(obj(1), them, 3)]) < even);
    // The race counts a creature by the life it threatens.
    let race = |life| {
        agent().foresight_features(&view(0, &[20, life], vec![permanent(obj(1), me, 4)]))[13]
    };
    assert!(race(4) > race(20));
    let mut lost = view(0, &[20, 0], vec![]);
    lost.seats[1].loss = Some(baylee_view::LossCause::Life);
    assert!((agent().foresight_value(&lost) - 1.0).abs() < f64::EPSILON);
}

/// A player who has shown nothing is supposed to play a deck like the
/// seat's own; one who has shown cards, those cards, lands at about two in
/// five, and nothing its colours cannot cast.
#[test]
fn the_prior_is_built_from_what_the_seat_sees() {
    let them = PlayerId::new(1);
    let own = vec![index("Island"), index("Island"), index("Coral Merfolk")];
    let nothing = public_prior(&view(0, &[20, 20], vec![]), &own);
    assert_eq!(nothing.len(), 1);
    assert_eq!(nothing[0].0, them);
    let weight = |list: &[(baylee_core::ids::CardIndex, u32)], name: &str| {
        list.iter()
            .find(|(card, _)| *card == index(name))
            .map_or(0, |(_, weight)| u64::from(*weight))
    };
    assert_eq!(nothing[0].1.len(), 2);
    assert_eq!(
        weight(&nothing[0].1, "Island"),
        2 * weight(&nothing[0].1, "Coral Merfolk")
    );

    let bears = carded(
        permanent(obj(1), them, 2),
        "Grizzly Bears",
        TypeSet::CREATURE,
    );
    let forest = carded(permanent(obj(2), them, 0), "Forest", TypeSet::LAND);
    // The seat's own permanent says nothing about them.
    let mine = carded(
        permanent(obj(3), PlayerId::new(0), 1),
        "Coral Merfolk",
        TypeSet::CREATURE,
    );
    let shown = public_prior(&view(0, &[20, 20], vec![bears, forest, mine]), &own);
    let list = &shown[0].1;
    let total: u64 = list.iter().map(|(_, weight)| u64::from(*weight)).sum();
    let lands: u64 = list
        .iter()
        .filter(|(card, _)| {
            baylee_cards::by_index(*card)
                .is_some_and(|def| def.faces[0].types.contains(TypeSet::LAND))
        })
        .map(|(_, weight)| u64::from(*weight))
        .sum();
    assert!(
        weight(list, "Grizzly Bears") * 3 > total,
        "the card shown comes again"
    );
    assert!(weight(list, "Forest") > 0);
    assert_eq!(weight(list, "Island"), 0);
    assert_eq!(
        weight(list, "Coral Merfolk"),
        0,
        "blue is not what they showed"
    );
    assert_eq!(weight(list, "Lightning Bolt"), 0);
    assert!(
        weight(list, "Llanowar Elves") > 0,
        "the pool's green cards at a low weight"
    );
    assert!(weight(list, "Llanowar Elves") * 100 < weight(list, "Grizzly Bears"));
    let share = lands * 100 / total;
    assert!(
        (35..=45).contains(&share),
        "lands are {share} % of the prior"
    );
    // The same view, the same prior.
    let again = public_prior(
        &view(
            0,
            &[20, 20],
            vec![
                carded(
                    permanent(obj(1), them, 2),
                    "Grizzly Bears",
                    TypeSet::CREATURE,
                ),
                carded(permanent(obj(2), them, 0), "Forest", TypeSet::LAND),
            ],
        ),
        &own,
    );
    assert_eq!(again, shown);
}

/// Every feature is one side less the other: the two seats of a duel read
/// the same table as each other's negative, so an even table is even with
/// no intercept.
#[test]
fn the_two_seats_read_a_table_as_each_others_negative() {
    use baylee_cards_dsl::KeywordSet;
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let mut flyer = permanent(obj(1), me, 2);
    flyer.keywords = KeywordSet::FLYING.bits() | KeywordSet::LIFELINK.bits();
    let mut wall = permanent(obj(2), them, 4);
    wall.keywords = KeywordSet::DEFENDER.bits();
    let mut tapped = permanent(obj(3), them, 3);
    tapped.status = ObjectStatus::TAPPED;
    let mut relic = permanent(obj(4), me, 0);
    relic.types = TypeSet::ARTIFACT;
    let board = vec![flyer, wall, tapped, relic, permanent(obj(5), me, 1)];
    let mut mine = view(0, &[17, 9], board.clone());
    mine.seats[0].hand_count = 3;
    mine.seats[1].library_count = 4;
    let mut theirs = view(1, &[17, 9], board);
    theirs.seats[0].hand_count = 3;
    theirs.seats[1].library_count = 4;
    theirs.active = mine.active;
    let (a, b) = (
        agent().foresight_features(&mine),
        agent().foresight_features(&theirs),
    );
    for (name, (a, b)) in crate::foresight::FEATURE_NAMES.iter().zip(a.iter().zip(b)) {
        assert!((a + b).abs() < 1e-12, "{name}: {a} and {b}");
    }
    let value = agent().foresight_value(&mine) + agent().foresight_value(&theirs);
    assert!((value - 1.0).abs() < 1e-12, "{value}");
}

/// What gets past the blockers: a flyer over creatures on the ground, and
/// an attack into creatures that are tapped while it is the attacker's own
/// turn; the side whose turn it is not is taken to untap before it blocks.
#[test]
fn damage_that_gets_through_is_counted_past_the_blockers() {
    use baylee_cards_dsl::KeywordSet;
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let through = |board: Vec<PublicObject>, active: PlayerId| {
        let mut v = view(0, &[20, 20], board);
        v.active = active;
        let at = crate::foresight::FEATURE_NAMES
            .iter()
            .position(|name| *name == "through")
            .expect("the feature");
        agent().foresight_features(&v)[at] * 20.0
    };
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let blocker = permanent(obj(2), them, 1);
    // One attacker, one blocker: mine is blocked; theirs is too.
    let ground = permanent(obj(1), me, 3);
    assert!(close(through(vec![ground.clone(), blocker.clone()], me), 0.0));
    // A flyer is not blocked from the ground, and their 1/1 is by it.
    let mut flyer = ground.clone();
    flyer.keywords = KeywordSet::FLYING.bits();
    assert!(close(through(vec![flyer, blocker.clone()], me), 3.0));
    // A tapped blocker blocks nothing on my turn, and untaps before theirs:
    // my 3 get through, and so does its 1 past my one blocker's block of
    // nothing else.
    let mut tapped = blocker;
    tapped.status = ObjectStatus::TAPPED;
    assert!(close(through(vec![ground.clone(), tapped.clone()], me), 3.0));
    assert!(close(through(vec![ground, tapped], them), 0.0));
}
