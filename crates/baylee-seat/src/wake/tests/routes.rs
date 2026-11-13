//! A card cast from a graveyard or from exile, reached at the price the
//! engine names for the way it is cast there (`LegalActions::cast_ways`):
//! a disturb face's cost (CR 702.146a), a foretell cost (CR 702.143a).
//!
//! The orders priced a graveyard card the engine offered at its front's mana
//! cost and an exiled one at its printed cost, so a Benevolent Geist was
//! offered beside two Islands that do not pay its `{2}{U}`, and a foretold
//! Doomskar was never offered beside the three Plains that pay `{1}{W}{W}`.

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{CastModeKind, LegalActions, Pending, PlayerAction, timeout_answer};
use baylee_gamehost::Session;
use baylee_view::{Phase, PlayerView};

use super::super::raised::Way;
use super::super::reachable;
use super::played::reach_of;

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool")),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

/// A table where seat 0 starts over `mine`, holding `hand`, both libraries
/// `library`.
fn table(library: &str, mine: &[&str], hand: &[&str]) -> Session {
    table_with_theirs(library, mine, hand, &[])
}

/// [`table`] with `theirs` on the opponent's battlefield.
fn table_with_theirs(library: &str, mine: &[&str], hand: &[&str], theirs: &[&str]) -> Session {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(41, entry(library).card).unwrap();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.deck = (0..40).map(|_| entry(library)).collect();
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    Session::new(&preset).unwrap()
}

/// Plays on, every question answered by `answer` or else its timeout answer,
/// until seat 0 holds priority with nothing on the stack in its first main
/// phase of turn `turn` or later.
fn play_to(
    session: &mut Session,
    turn: u32,
    answer: &mut dyn FnMut(&Pending) -> Option<PlayerAction>,
) -> (PlayerView, LegalActions) {
    for _ in 0..400 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        if let Pending::Priority { player, .. } = pending
            && player == ME
        {
            let view = session.observe_view(ME);
            if view.phase == Phase::FirstMain
                && view.active == ME
                && view.turn >= turn
                && view.stack.is_empty()
            {
                let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
                let Pending::Priority { legal, .. } = pending else {
                    unreachable!("seat 0 holds priority");
                };
                return (view, *legal);
            }
        }
        let action = answer(&pending)
            .or_else(|| timeout_answer(&pending))
            .unwrap_or_else(|| panic!("no answer to {pending:?}"));
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("seat 0's main phase of turn {turn} was not reached");
}

fn id_of(view: &PlayerView, name: &str) -> ObjectId {
    view.hand
        .iter()
        .find(|card| card.name == name)
        .map(|card| card.id)
        .or_else(|| {
            view.battlefield
                .iter()
                .find(|o| o.controller == ME && o.name.as_ref() == name)
                .map(|o| o.id)
        })
        .unwrap_or_else(|| panic!("{name} is not in seat 0's hand or on its battlefield"))
}

/// Malevolent Hermit discarded to Faithless Looting: two Islands do not pay
/// Benevolent Geist's disturb `{2}{U}`, three do, and the reach names the
/// face the engine casts.
#[test]
fn a_disturb_card_is_reached_at_its_disturb_cost() {
    for (islands, reached) in [(2, false), (3, true)] {
        let mut mine = vec!["Mountain"];
        mine.extend(std::iter::repeat_n("Island", islands));
        let mut session = table(
            "Mountain",
            &mine,
            &["Faithless Looting", "Malevolent Hermit"],
        );
        let (view, _) = play_to(&mut session, 1, &mut |_| None);
        let hermit = id_of(&view, "Malevolent Hermit");
        let looting = id_of(&view, "Faithless Looting");
        let mountain = id_of(&view, "Mountain");
        session
            .act_slice(
                ME,
                PlayerAction::ActivateManaAbility { source: mountain },
                0,
            )
            .unwrap();
        session
            .act_slice(ME, PlayerAction::CastSpell { card: looting }, 0)
            .unwrap();
        let (view, legal) = play_to(&mut session, 1, &mut |pending| match pending {
            Pending::ChooseCards { options, min, .. } if options.contains(&hermit) => {
                Some(PlayerAction::ChooseObjects {
                    objects: std::iter::once(hermit)
                        .chain(options.iter().copied().filter(|id| *id != hermit))
                        .take(usize::from(*min))
                        .collect(),
                })
            }
            Pending::ChooseCards { options, min, .. } => Some(PlayerAction::ChooseObjects {
                objects: options.iter().copied().take(usize::from(*min)).collect(),
            }),
            _ => None,
        });
        assert!(
            view.graveyards[0].iter().any(|o| o.id == hermit),
            "the Hermit was discarded"
        );
        let reach = reachable(&view, &legal);
        let geist = reach_of(&reach, "Malevolent Hermit");
        assert_eq!(
            geist.is_some(),
            reached,
            "{islands} Islands and Benevolent Geist's {{2}}{{U}}: {reach:?}"
        );
        if let Some(geist) = geist {
            assert_eq!(geist.plan.steps.len(), 3);
            assert_eq!(geist.way, Some(Way::Mode(CastModeKind::Face(1))));
        }
    }
}

/// Doomskar foretold on turn 1 is reached on turn 3 for its foretell cost:
/// three Plains, which do not pay its printed `{3}{W}{W}`.
#[test]
fn a_foretold_card_is_reached_at_its_foretell_cost() {
    let mut session = table("Plains", &["Plains", "Plains", "Plains"], &["Doomskar"]);
    let (view, _) = play_to(&mut session, 1, &mut |_| None);
    let doomskar = id_of(&view, "Doomskar");
    for plains in view
        .battlefield
        .iter()
        .filter(|o| o.controller == ME)
        .take(2)
        .map(|o| o.id)
    {
        session
            .act_slice(ME, PlayerAction::ActivateManaAbility { source: plains }, 0)
            .unwrap();
    }
    session
        .act_slice(ME, PlayerAction::Foretell { card: doomskar }, 0)
        .unwrap();
    let (view, legal) = play_to(&mut session, 3, &mut |_| None);
    let reach = reachable(&view, &legal);
    let foretold = reach_of(&reach, "Doomskar").expect("three Plains pay {1}{W}{W}");
    assert_eq!(foretold.plan.steps.len(), 3);
    assert_eq!(foretold.way, Some(Way::Mode(CastModeKind::Foretell)));
}

/// Two Ancient Ziggurats ("Spend this mana only to cast a creature spell")
/// are Grizzly Bears' {1}{G}, and the seat reaches it with both; Lightning
/// Bolt it never reaches with them (CR 106.6). The engine says which spells
/// their mana pays (`LegalActions::restricted_taps`).
#[test]
fn restricted_mana_is_reached_for_the_spells_it_pays() {
    let mut session = table(
        "Mountain",
        &["Ancient Ziggurat", "Ancient Ziggurat"],
        &["Grizzly Bears", "Lightning Bolt"],
    );
    let (view, legal) = play_to(&mut session, 1, &mut |_| None);
    let reach = reachable(&view, &legal);
    let bears = reach_of(&reach, "Grizzly Bears").expect("two Ziggurats pay {1}{G}");
    assert_eq!(bears.plan.steps.len(), 2);
    assert!(reach_of(&reach, "Lightning Bolt").is_none(), "{reach:?}");
}

/// A Ziggurat's red already floating and a Forest: the Bears take the
/// Forest's one tap, the floating red counted as the engine says it may pay
/// (`LegalActions::restricted_floating`).
#[test]
fn floating_restricted_mana_counts_for_the_spells_it_pays() {
    let mut session = table(
        "Forest",
        &["Ancient Ziggurat", "Forest"],
        &["Grizzly Bears"],
    );
    let (view, _) = play_to(&mut session, 1, &mut |_| None);
    let ziggurat = id_of(&view, "Ancient Ziggurat");
    session
        .act_slice(
            ME,
            PlayerAction::ActivateAbility {
                source: ziggurat,
                ability_index: 0,
            },
            0,
        )
        .unwrap();
    let (view, legal) = play_to(&mut session, 1, &mut |pending| match pending {
        Pending::ChooseColor { .. } => {
            Some(PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Red))
        }
        _ => None,
    });
    let reach = reachable(&view, &legal);
    let bears = reach_of(&reach, "Grizzly Bears").expect("red floating and a Forest");
    assert_eq!(bears.plan.steps.len(), 1);
}

/// Dismember, `{1}{B/P}{B/P}`, at 20 life beside a Mountain: the Mountain
/// pays `{1}` and 4 life the two Phyrexian symbols (CR 107.4f), as the
/// engine takes it. The seat planned the symbols as black only and never
/// offered it (Carbon's reports, 11.11.2026).
#[test]
fn phyrexian_mana_is_reached_with_life() {
    let (_, view, legal) =
        super::played::position(&["Mountain"], &["Dismember"], &["Grizzly Bears"]);
    let reach = reachable(&view, &legal);
    let dismember = reach_of(&reach, "Dismember").expect("a Mountain and 4 life pay {1}{B/P}{B/P}");
    assert_eq!(dismember.plan.steps.len(), 1);
}

/// Attacks with Gwenom and resolves its trigger, then stops in seat 0's
/// second main phase. The permission under test always comes from that real
/// attack, never from a hand-written view or legal-action list.
fn after_gwenom_attack(session: &mut Session) -> (PlayerView, LegalActions) {
    let mut attacked = false;
    for _ in 0..600 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        if let Pending::Priority { player, .. } = pending
            && player == ME
        {
            let view = session.observe_view(ME);
            if attacked
                && view.phase == Phase::SecondMain
                && view.active == ME
                && view.stack.is_empty()
            {
                let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
                let Pending::Priority { legal, .. } = pending else {
                    unreachable!("seat 0 holds priority");
                };
                return (view, *legal);
            }
        }
        let action = match &pending {
            Pending::ChooseAttackers { attackers, .. } if player == ME => {
                let view = session.observe_view(ME);
                let gwenom = attackers
                    .iter()
                    .copied()
                    .find(|id| {
                        view.object(*id)
                            .is_some_and(|object| object.name == "Gwenom, Remorseless")
                    })
                    .expect("Gwenom can attack");
                attacked = true;
                PlayerAction::DeclareAttackers {
                    attackers: vec![(gwenom, baylee_core::ids::Defender::Player(PlayerId::new(1)))],
                }
            }
            _ => timeout_answer(&pending).unwrap_or_else(|| panic!("no answer to {pending:?}")),
        };
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("Gwenom's second main phase was not reached");
}

/// With no mana source at all, the top Dark Ritual is still a real play for
/// one life. The narrator offers the named route once, with no generic cast
/// and no empty tap plan, and answering it casts through the engine's route.
#[test]
fn a_zero_mana_library_route_survives_the_whole_seat_path() {
    use crate::narrator::{Act, Decision, Narrator};

    let mut session = table("Dark Ritual", &["Gwenom, Remorseless"], &[]);
    let (view, legal) = after_gwenom_attack(&mut session);
    let life_before = view.seat(ME).unwrap().life;
    let reach = reachable(&view, &legal);
    let ritual = reach_of(&reach, "Dark Ritual").expect("one life needs no mana");
    assert_eq!(ritual.from, super::super::ReachFrom::LibraryTop);
    assert!(ritual.plan.steps.is_empty());
    assert_eq!(ritual.plan.cost, baylee_core::mana::ManaCost::ZERO);
    assert!(matches!(
        ritual.way,
        Some(Way::Mode(CastModeKind::LibraryTopLife {
            life: 1,
            face: 0,
            kicked: false,
            ..
        }))
    ));

    let request = crate::narrator::tests::request(
        view,
        Pending::Priority {
            player: ME,
            legal: Box::new(legal),
        },
        baylee_view::LogTail::default(),
    );
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let offered: Vec<_> = wake
        .menu
        .offered()
        .iter()
        .filter(|option| {
            option.text.contains("Dark Ritual") && option.text.contains("paying 1 life")
        })
        .collect();
    assert_eq!(offered.len(), 1, "{:#?}", wake.text);
    assert!(!offered[0].text.contains("(taps )"), "{}", wake.text);
    assert!(
        !wake.text.contains("Dark Ritual (paid from"),
        "{}",
        wake.text
    );
    let resolved = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![offered[0].id.clone()],
            ..Default::default()
        })
        .expect("the named library route answers the menu");
    let Act::Now(PlayerAction::CastSpell { card }) = resolved.act else {
        panic!("a zero-mana route casts directly: {:?}", resolved.act);
    };
    session
        .act_slice(ME, PlayerAction::CastSpell { card }, 0)
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = session.pending().clone() {
        let route = options
            .iter()
            .position(|option| matches!(option.kind, CastModeKind::LibraryTopLife { life: 1, .. }))
            .expect("the life route is offered");
        session
            .act_slice(ME, PlayerAction::ChooseMode(route), 0)
            .unwrap();
    }
    let view = session.observe_view(ME);
    assert_eq!(view.seat(ME).unwrap().life, life_before - 1);
    assert!(view.stack.iter().any(|object| object.id == card));
}

/// An Adventure's creature and spell faces both have no remaining mana, but
/// they are different routes and pay different life. Choosing Petty Theft
/// retains that face and its target instead of collapsing into the front.
#[test]
fn equal_mana_library_faces_remain_distinct_and_cast_the_chosen_face() {
    let mut session = table_with_theirs(
        "Brazen Borrower",
        &["Gwenom, Remorseless"],
        &[],
        &["Grizzly Bears"],
    );
    let (view, legal) = after_gwenom_attack(&mut session);
    let life_before = view.seat(ME).unwrap().life;
    let reach = reachable(&view, &legal);
    let mut routes: Vec<_> = reach
        .iter()
        .filter(|reach| reach.card.index == entry("Brazen Borrower").card)
        .collect();
    routes.sort_by_key(|reach| match reach.way {
        Some(Way::Mode(CastModeKind::LibraryTopLife { life, .. })) => life,
        _ => u32::MAX,
    });
    assert_eq!(routes.len(), 2, "{routes:?}");
    assert!(routes.iter().all(|reach| reach.plan.steps.is_empty()));
    assert!(matches!(
        routes[0].way,
        Some(Way::Mode(CastModeKind::LibraryTopLife {
            life: 2,
            face: 1,
            ..
        }))
    ));
    assert!(matches!(
        routes[1].way,
        Some(Way::Mode(CastModeKind::LibraryTopLife {
            life: 3,
            face: 0,
            ..
        }))
    ));

    let card = routes[0].object;
    super::played::run(
        &mut session,
        &routes[0].plan,
        PlayerAction::CastSpell { card },
    );
    let Pending::ChooseCastMode { options, .. } = session.pending().clone() else {
        panic!("the route question follows: {:?}", session.pending());
    };
    let route = options
        .iter()
        .position(|option| {
            matches!(
                option.kind,
                CastModeKind::LibraryTopLife {
                    life: 2,
                    face: 1,
                    ..
                }
            )
        })
        .expect("Petty Theft is still offered");
    session
        .act_slice(ME, PlayerAction::ChooseMode(route), 0)
        .unwrap();
    let Pending::ChooseTargets { options, .. } = session.pending().clone() else {
        panic!("Petty Theft asks its own target: {:?}", session.pending());
    };
    let target = *options.first().expect("the Bears are a target");
    session
        .act_slice(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: Vec::new(),
            },
            0,
        )
        .unwrap();
    let view = session.observe_view(ME);
    assert_eq!(view.seat(ME).unwrap().life, life_before - 2);
    let spell = view.stack.iter().find(|object| object.id == card).unwrap();
    assert_eq!(spell.card.unwrap().face, 1);
}

/// The life payment replaces only the printed mana cost. Kicker's remaining
/// {5} is planned and tapped, while the unkicked route stays mana-free.
#[test]
fn a_library_kicker_taps_only_the_remaining_mana() {
    let mut session = table_with_theirs(
        "Rite of Replication",
        &[
            "Gwenom, Remorseless",
            "Island",
            "Island",
            "Island",
            "Island",
            "Island",
        ],
        &[],
        &["Grizzly Bears"],
    );
    let (view, legal) = after_gwenom_attack(&mut session);
    let life_before = view.seat(ME).unwrap().life;
    let reach = reachable(&view, &legal);
    let routes: Vec<_> = reach
        .iter()
        .filter(|reach| reach.card.index == entry("Rite of Replication").card)
        .collect();
    assert_eq!(routes.len(), 2, "{routes:?}");
    let kicked = routes
        .iter()
        .find(|reach| {
            matches!(
                reach.way,
                Some(Way::Mode(CastModeKind::LibraryTopLife { kicked: true, .. }))
            )
        })
        .expect("the kicked route is offered");
    assert_eq!(kicked.plan.cost, baylee_core::mana::ManaCost::parse("{5}"));
    assert_eq!(kicked.plan.steps.len(), 5);
    assert!(matches!(
        kicked.way,
        Some(Way::Mode(CastModeKind::LibraryTopLife { life: 4, .. }))
    ));
    assert!(routes.iter().any(|reach| reach.plan.steps.is_empty()));

    let card = kicked.object;
    super::played::run(&mut session, &kicked.plan, PlayerAction::CastSpell { card });
    let Pending::ChooseCastMode { options, .. } = session.pending().clone() else {
        panic!("the route question follows: {:?}", session.pending());
    };
    let route = options
        .iter()
        .position(|option| {
            matches!(
                option.kind,
                CastModeKind::LibraryTopLife {
                    life: 4,
                    kicked: true,
                    ..
                }
            )
        })
        .expect("the kicked route is offered");
    session
        .act_slice(ME, PlayerAction::ChooseMode(route), 0)
        .unwrap();
    let Pending::ChooseTargets { options, .. } = session.pending().clone() else {
        panic!("the Rite asks its target: {:?}", session.pending());
    };
    session
        .act_slice(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![options[0]],
                players: Vec::new(),
            },
            0,
        )
        .unwrap();
    let view = session.observe_view(ME);
    assert_eq!(
        view.seat(ME).unwrap().life,
        life_before - 4,
        "after the kicked target: {:?}",
        session.pending()
    );
    assert_eq!(view.seat(ME).unwrap().mana_pool.total(), 0);
    assert!(view.stack.iter().any(|object| object.id == card));
}
