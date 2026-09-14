use super::*;
use crate::test_support::{ViewBuilder, printed, token};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_view::{AttackerView, BlockerView, CardIdentity};

const WIDE: f32 = 40.0;

/// A row barely wider than one card, which is where merging lives.
///
/// Identical permanents merge only once they would have to overlap, so a
/// test *about* merging has to be given a row that cannot hold its cards
/// — otherwise it draws them all separately and asserts nothing. Every
/// test below that is about what stays apart when things merge uses this
/// rather than [`WIDE`].
const CROWDED: f32 = 2.0;

/// Nobody may look through a library, their own included (CR 401.2), so
/// the pile beside the mat is inert rather than merely empty — and it
/// stays inert with sixty cards in it, which is the case an "is it empty"
/// reading would get wrong.
#[test]
fn a_library_is_never_browsable_however_full_it_is() {
    let mut library = ZonePile::empty(PileKind::Library);
    library.count = 60;
    assert!(!library.is_browsable());

    let mut graveyard = ZonePile::empty(PileKind::Graveyard);
    assert!(!graveyard.is_browsable(), "an empty pile opens nothing");
    graveyard.count = 1;
    assert!(graveyard.is_browsable());
}

/// What a hover spreads out of a pile, and what it may never spread out
/// of a library.
mod fan {
    use super::*;

    fn pile(view: &baylee_view::PlayerView, kind: PileKind) -> ZonePile {
        zone_piles(view, PlayerId::new(0))
            .into_iter()
            .find(|p| p.kind == kind)
            .expect("the seat has this pile")
    }

    /// Top of the pile first, and never more than seven — a graveyard of
    /// ten fans its last seven, newest first.
    ///
    /// `ZonePosition::Top` pushes, so the object listed *last* is the one
    /// lying on top; the fan reverses that, which is the whole of the
    /// ordering claim. It is asserted against the names rather than
    /// against a length, because a fan that took the first seven would
    /// also be seven cards long and would be the wrong seven.
    #[test]
    fn a_graveyard_fans_its_newest_seven_newest_first() {
        let dead: Vec<_> = (0..10)
            .map(|i| printed(i, 0, &format!("card {i}"), u16::try_from(i).unwrap() + 1))
            .collect();
        let view = ViewBuilder::new(2).with_graveyard(0, dead).build();
        let graveyard = pile(&view, PileKind::Graveyard);

        assert_eq!(graveyard.count, 10);
        assert_eq!(graveyard.fan_len(), ZonePile::FAN_MAX);
        assert_eq!(
            graveyard
                .fan
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            [
                "card 9", "card 8", "card 7", "card 6", "card 5", "card 4", "card 3"
            ],
            "the fan is not the newest seven, newest first"
        );
        assert_eq!(
            graveyard.fan.first().map(|c| c.object),
            graveyard.top,
            "the card on top of the pile is the card at the top of the fan"
        );
    }

    /// A pile shallower than the fan fans what it has, and an empty one
    /// fans nothing at all.
    #[test]
    fn a_short_pile_fans_what_it_has() {
        let view = ViewBuilder::new(2)
            .with_exile(0, vec![printed(1, 0, "Oblivion Ring", 4)])
            .build();
        let exile = pile(&view, PileKind::Exile);
        assert_eq!(exile.fan_len(), 1);
        assert_eq!(exile.fan.len(), 1);

        let empty = ZonePile::empty(PileKind::Graveyard);
        assert_eq!(empty.fan_len(), 0);
        assert!(empty.fan.is_empty());
    }

    /// The second reading of CR 401.2, and the one this model enforces by
    /// construction: a library fans, and has nothing to fan.
    ///
    /// [`ZonePile::fan`] is empty for a library not because a rule here
    /// empties it but because a `PlayerView` carries a library as a
    /// *count* — there are no cards in it to put in the list. What the
    /// fan draws there is [`ZonePile::fan_len`] card backs, which say how
    /// deep the pile is and nothing else. The counter-test is the
    /// graveyard above: same code, same seat, seven faces.
    #[test]
    fn a_library_fans_backs_and_never_faces() {
        let view = ViewBuilder::new(2).build();
        let library = pile(&view, PileKind::Library);

        assert_eq!(library.count, 80, "the builder deals a full library");
        assert_eq!(
            library.fan_len(),
            ZonePile::FAN_MAX,
            "a library fans like any other pile"
        );
        assert!(
            library.fan.is_empty(),
            "a library handed the fan a face to draw"
        );
        assert!(library.art.is_none() && library.top.is_none());
    }

    /// A token in a graveyard is a slot in the fan with no picture, not a
    /// card the fan skips.
    ///
    /// It is really lying there — a token that has left the battlefield
    /// ceases to exist only when state-based actions are next checked
    /// (CR 111.7) — and a fan that dropped it would say the pile is
    /// shallower than it is, on exactly the frame a player is looking to
    /// see what just died.
    #[test]
    fn a_token_in_the_graveyard_is_a_blank_slot_and_not_a_gap() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![printed(1, 0, "Llanowar Elves", 3), token(2, 0, "Elf", 1, 1)],
            )
            .build();
        let graveyard = pile(&view, PileKind::Graveyard);

        assert_eq!(graveyard.fan.len(), 2, "the token was dropped from the fan");
        assert_eq!(graveyard.fan[0].name, "Elf");
        assert!(graveyard.fan[0].art.is_none(), "a token has no printing");
        assert!(graveyard.fan[1].art.is_some());
    }

    /// A hover opens the pile the card is in, at the seat it belongs to.
    ///
    /// A graveyard is public, so an opponent's opens like anyone's — and
    /// it has to open as *theirs*, because the fan stands beside their
    /// mat and not beside the viewer's.
    #[test]
    fn a_hover_opens_the_pile_the_card_is_lying_in() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![printed(1, 0, "buried", 1), printed(3, 0, "on top", 3)],
            )
            .with_exile(1, vec![printed(2, 1, "theirs", 2)])
            .build();
        let model = model(&view);
        let card_in = |seat: u8, kind, at: usize| {
            model
                .pod(PlayerId::new(seat))
                .expect("the seat is at the table")
                .piles
                .iter()
                .find(|p| p.kind == kind)
                .expect("the seat has this pile")
                .fan[at]
                .object
        };

        assert_eq!(
            model.fanned_pile(Some(card_in(0, PileKind::Graveyard, 0))),
            Some((PlayerId::new(0), PileKind::Graveyard))
        );
        // And the card *under* that one, which is what the pointer is
        // over once the fan is out — and what a reading that knew only
        // the pile's top card would answer nothing for.
        assert_eq!(
            model.fanned_pile(Some(card_in(0, PileKind::Graveyard, 1))),
            Some((PlayerId::new(0), PileKind::Graveyard)),
            "the fan shut under a pointer that had travelled along it"
        );
        assert_eq!(
            model.fanned_pile(Some(card_in(1, PileKind::Exile, 0))),
            Some((PlayerId::new(1), PileKind::Exile)),
            "an opponent's exile opened as somebody else's pile"
        );
        assert_eq!(model.fanned_pile(None), None);
        assert_eq!(
            model.fanned_pile(Some(ObjectId::new(99, 0))),
            None,
            "a card lying on nothing opened a pile"
        );
    }

    /// Every face the fan will draw is resident before the hover, and
    /// each is asked for once.
    ///
    /// A fan is a hover and a hover has no frame to spare for a fetch,
    /// which is the same reason the pile's own top card is in this list.
    /// The dedup is the second half: the top card is in the fan *and* in
    /// `ZonePile::art`, so a list that did not dedup would ask for it
    /// twice.
    #[test]
    fn the_whole_fan_is_resident_before_the_hover() {
        let dead: Vec<_> = (0..3)
            .map(|i| printed(i, 0, &format!("card {i}"), u16::try_from(i).unwrap() + 1))
            .collect();
        let view = ViewBuilder::new(2).with_graveyard(0, dead).build();
        let keys = model(&view).required_images();
        let graveyard = pile(&view, PileKind::Graveyard);

        for card in &graveyard.fan {
            let key = card.art.expect("every one of these is a printed card");
            assert_eq!(
                keys.iter().filter(|k| **k == key).count(),
                1,
                "{} is not asked for exactly once",
                card.name
            );
        }
        assert_eq!(keys.len(), 3);
    }
}

/// The command zone is one zone drawn as one place per commander, and a
/// seat that has none is drawn no place at all.
///
/// Three claims, and the middle one is the only one a reader might not
/// expect: nothing *reflows*. A pile stands where its own kind stands, so
/// a seat with a single commander shows one slot and bare table where the
/// second would be, rather than sliding the graveyard over to close the
/// gap. Commanders are fixed before the first turn (CR 903.3), so a
/// seat's set of slots is the same on the last turn as on the first.
mod command_slots {
    use super::*;

    fn slots(view: &baylee_view::PlayerView) -> Vec<PileKind> {
        zone_piles(view, PlayerId::new(0))
            .into_iter()
            .map(|p| p.kind)
            .collect()
    }

    #[test]
    fn a_seat_with_no_commander_is_drawn_no_command_zone() {
        let view = ViewBuilder::new(2).build();
        assert_eq!(
            slots(&view),
            vec![PileKind::Library, PileKind::Graveyard, PileKind::Exile],
            "a deck with no commander has no zone to draw"
        );
    }

    #[test]
    fn one_commander_is_one_slot_and_two_are_two() {
        let first = printed(1, 0, "Sidar Kondo", 11);
        let second = printed(2, 0, "Tana", 12);
        let one = ViewBuilder::new(2)
            .with_commanders(0, &[&first])
            .with_command(0, vec![first.clone()])
            .build();
        assert_eq!(
            slots(&one),
            vec![
                PileKind::Library,
                PileKind::Graveyard,
                PileKind::Exile,
                PileKind::Command
            ]
        );

        let two = ViewBuilder::new(2)
            .with_commanders(0, &[&first, &second])
            .with_command(0, vec![first.clone(), second.clone()])
            .build();
        assert_eq!(
            slots(&two),
            vec![
                PileKind::Library,
                PileKind::Graveyard,
                PileKind::Exile,
                PileKind::Command,
                PileKind::Command2
            ]
        );
    }

    /// Each partner lies on its own slot, and the second one is matched
    /// by handle rather than by position: an `ObjectId` survives the
    /// moves that make a card a new object (CR 400.7), so a partner that
    /// dies, goes home and comes back down lands on the slot it left.
    #[test]
    fn each_partner_lies_on_its_own_slot() {
        let first = printed(1, 0, "Sidar Kondo", 11);
        let second = printed(2, 0, "Tana", 12);
        // Listed second-first, which is what a command zone looks like
        // after the first one has been cast and has come back.
        let view = ViewBuilder::new(2)
            .with_commanders(0, &[&first, &second])
            .with_command(0, vec![second.clone(), first.clone()])
            .build();
        let piles = zone_piles(&view, PlayerId::new(0));
        let at = |kind| {
            piles
                .iter()
                .find(|p| p.kind == kind)
                .expect("the slot is drawn")
                .clone()
        };
        assert_eq!(at(PileKind::Command).top, Some(first.id));
        assert_eq!(at(PileKind::Command).count, 1);
        assert_eq!(at(PileKind::Command2).top, Some(second.id));
        assert_eq!(at(PileKind::Command2).count, 1);
    }

    /// A commander that is on the battlefield leaves its slot empty, and
    /// the slot is still there: the zone exists whether or not a card is
    /// in it, the commander can return to it (CR 903.9), and the
    /// uncovered mark is exactly the signal "your commander is out".
    #[test]
    fn a_commander_on_the_battlefield_leaves_its_slot_standing_and_empty() {
        let first = printed(1, 0, "Sidar Kondo", 11);
        let view = ViewBuilder::new(2).with_commanders(0, &[&first]).build();
        let piles = zone_piles(&view, PlayerId::new(0));
        let command = piles
            .iter()
            .find(|p| p.kind == PileKind::Command)
            .expect("the slot is still drawn");
        assert_eq!(command.count, 0);
        assert_eq!(command.top, None);
    }

    /// An emblem belongs to the first slot. It is in the command zone and
    /// it is not a commander, so it goes where a seat with one commander
    /// already looks.
    #[test]
    fn what_is_not_a_commander_lies_on_the_first_slot() {
        let first = printed(1, 0, "Sidar Kondo", 11);
        let second = printed(2, 0, "Tana", 12);
        let emblem = token(3, 0, "Emblem", 0, 0);
        let view = ViewBuilder::new(2)
            .with_commanders(0, &[&first, &second])
            .with_command(0, vec![first.clone(), emblem.clone(), second.clone()])
            .build();
        let piles = zone_piles(&view, PlayerId::new(0));
        let count = |kind| {
            piles
                .iter()
                .find(|p| p.kind == kind)
                .expect("the slot is drawn")
                .count
        };
        assert_eq!(count(PileKind::Command), 2, "the commander and the emblem");
        assert_eq!(count(PileKind::Command2), 1);
    }
}

fn model(view: &PlayerView) -> BoardModel {
    BoardModel::from_view(view, Openings::none(), |_| WIDE, Registry::none())
}

fn crowded_model(view: &PlayerView) -> BoardModel {
    BoardModel::from_view(view, Openings::none(), |_| CROWDED, Registry::none())
}

#[test]
fn keyword_bits_match_the_card_dsl() {
    // If the DSL renumbers a keyword, this fails rather than silently
    // drawing the wrong icon on every card in the game.
    use baylee_cards_dsl::KeywordSet as K;
    assert_eq!(keyword_bits::FLYING, K::FLYING.bits());
    assert_eq!(keyword_bits::DEATHTOUCH, K::DEATHTOUCH.bits());
    assert_eq!(keyword_bits::TRAMPLE, K::TRAMPLE.bits());
    assert_eq!(keyword_bits::VIGILANCE, K::VIGILANCE.bits());
    assert_eq!(keyword_bits::DEFENDER, K::DEFENDER.bits());
    assert_eq!(keyword_bits::INDESTRUCTIBLE, K::INDESTRUCTIBLE.bits());
}

#[test]
fn identical_tokens_collapse_into_one_counted_card() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, (0..12).map(|i| token(i, 0, "Soldier", 1, 1)))
        .build();
    let m = crowded_model(&view);
    let pod = m.pod(PlayerId::new(0)).expect("pod");
    let lane = pod.lane(LaneKind::Creatures).expect("creature lane");

    assert_eq!(lane.groups.len(), 1, "twelve identical tokens draw as one");
    assert_eq!(lane.groups[0].count(), 12);
    assert!(lane.groups[0].is_stack());
    assert_eq!(lane.permanent_count(), 12);
}

#[test]
fn a_tapped_token_does_not_hide_inside_the_untapped_stack() {
    let mut objs: Vec<PublicObject> = (0..5).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[3].status = ObjectStatus::TAPPED;
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = crowded_model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");

    // Four untapped plus one tapped: whether a blocker is available is
    // exactly what a player is reading the board for.
    assert_eq!(lane.groups.len(), 2);
    let counts: Vec<usize> = lane.groups.iter().map(CardGroup::count).collect();
    assert!(counts.contains(&4) && counts.contains(&1));
}

#[test]
fn attacking_and_blocking_creatures_never_merge() {
    let objs: Vec<PublicObject> = (0..6).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    let attacker = objs[0].id;
    let blocker = objs[1].id;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, objs)
        .with_combat(
            vec![AttackerView {
                creature: attacker,
                defending: baylee_core::ids::Defender::Player(PlayerId::new(1)),
                blocked: true,
            }],
            vec![BlockerView { blocker, attacker }],
        )
        .build();
    let m = crowded_model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");

    // Four fungible tokens in one group, plus the attacker and the blocker
    // as their own cards.
    assert_eq!(lane.groups.len(), 3);
    assert_eq!(lane.permanent_count(), 6);
    let reasons: Vec<Option<Individual>> = lane.groups.iter().map(|g| g.individual).collect();
    assert!(reasons.contains(&Some(Individual::Blocked)));
    assert!(reasons.contains(&Some(Individual::Blocking)));
}

#[test]
fn an_enchanted_creature_and_its_aura_both_stay_individual() {
    let mut objs: Vec<PublicObject> = (0..4).map(|i| token(i, 0, "Bear", 2, 2)).collect();
    let host = objs[0].id;
    let mut aura = token(90, 0, "Rancor", 0, 0);
    aura.types = TypeSet::ENCHANTMENT;
    aura.attached_to = Some(host);
    objs.push(aura);
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = crowded_model(&view);
    let pod = m.pod(PlayerId::new(0)).expect("pod");

    let creatures = pod.lane(LaneKind::Creatures).expect("creatures");
    // Three plain bears group; the enchanted one is separate.
    assert_eq!(creatures.groups.len(), 2);
    assert!(
        creatures
            .groups
            .iter()
            .any(|g| g.individual == Some(Individual::HasAttachments))
    );

    let support = pod.lane(LaneKind::Support).expect("support");
    assert_eq!(support.groups.len(), 1);
    assert_eq!(support.groups[0].individual, Some(Individual::Attached));
}

#[test]
fn a_targeted_permanent_is_pulled_out_of_its_group() {
    let objs: Vec<PublicObject> = (0..5).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    let victim = objs[2].id;
    let mut bolt = token(50, 1, "Lightning Bolt", 0, 0);
    bolt.types = TypeSet::INSTANT;
    bolt.targets = vec![TargetRef::Object(victim)];
    let view = ViewBuilder::new(2)
        .with_battlefield(0, objs)
        .with_stack(vec![bolt])
        .build();
    let m = crowded_model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");

    assert_eq!(lane.groups.len(), 2);
    assert!(
        lane.groups
            .iter()
            .any(|g| g.individual == Some(Individual::Targeted) && g.count() == 1)
    );
}

#[test]
fn permanents_land_in_the_lane_a_player_looks_for_them_in() {
    let mut creature_land = token(1, 0, "Dryad Arbor", 1, 1);
    creature_land.types = TypeSet::LAND.union(TypeSet::CREATURE);
    let mut plain_land = token(2, 0, "Forest", 0, 0);
    plain_land.types = TypeSet::LAND;
    let mut artifact = token(3, 0, "Treasure", 0, 0);
    artifact.types = TypeSet::ARTIFACT;

    assert_eq!(lane_of(creature_land.types), LaneKind::Creatures);
    assert_eq!(lane_of(plain_land.types), LaneKind::Lands);
    assert_eq!(lane_of(artifact.types), LaneKind::Support);
}

#[test]
fn token_chips_summarise_a_wide_board_as_text() {
    let mut objs: Vec<PublicObject> = (0..12).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[0].status = ObjectStatus::TAPPED;
    objs.extend((20..23).map(|i| {
        let mut t = token(i, 0, "Treasure", 0, 0);
        t.types = TypeSet::ARTIFACT;
        t.power = None;
        t.toughness = None;
        t
    }));
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    let pod = m.pod(PlayerId::new(0)).expect("pod");

    assert_eq!(pod.tokens.len(), 2);
    assert_eq!(pod.tokens[0].label(), "12× 1/1 Soldier");
    assert_eq!(pod.tokens[0].tapped, 1);
    assert_eq!(pod.tokens[1].label(), "3× Treasure");
}

#[test]
fn threat_summary_counts_what_can_actually_swing() {
    let mut objs = vec![
        token(1, 0, "Bear", 2, 2),
        token(2, 0, "Bear", 2, 2),
        token(3, 0, "Wall", 0, 4),
        token(4, 0, "Bear", 2, 2),
    ];
    objs[1].status = ObjectStatus::TAPPED;
    objs[2].keywords = keyword_bits::DEFENDER;
    objs[3].summoning_sick = true;
    let mut land = token(5, 0, "Forest", 0, 0);
    land.types = TypeSet::LAND;
    land.power = None;
    land.toughness = None;
    objs.push(land);

    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    let t = m.pod(PlayerId::new(0)).expect("pod").threat;

    // Only the one untapped, non-sick, non-defender bear can attack now.
    assert_eq!(t.attack_power, 2);
    // Untapped and not a defender: the ready bear and the sick one.
    assert_eq!(t.potential_attackers, 2);
    // Blockers include the wall.
    assert_eq!(t.potential_blockers, 3);
    assert_eq!(t.open_mana, 1);
}

#[test]
fn air_defence_counts_flying_and_reach() {
    let mut objs = vec![
        token(1, 0, "Bird", 1, 1),
        token(2, 0, "Spider", 1, 3),
        token(3, 0, "Bear", 2, 2),
    ];
    objs[0].keywords = keyword_bits::FLYING;
    objs[1].keywords = keyword_bits::REACH;
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    assert_eq!(m.pod(PlayerId::new(0)).expect("pod").threat.air_defence, 2);
}

#[test]
fn phased_out_permanents_leave_the_board_entirely() {
    let mut objs: Vec<PublicObject> = (0..3).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[0].status = ObjectStatus::from_bits(ObjectStatus::PHASED_OUT.bits());
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    assert_eq!(m.pod(PlayerId::new(0)).expect("pod").permanent_count(), 2);
}

#[test]
fn pods_are_ordered_local_first_then_clockwise_in_turn_order() {
    let mut view = ViewBuilder::new(4).build();
    view.seat = PlayerId::new(2);
    let m = model(&view);
    let order: Vec<u8> = m.pods.iter().map(|p| p.player.get()).collect();
    assert_eq!(order, vec![2, 3, 0, 1]);
    assert!(m.pods[0].is_local);
    assert!(!m.pods[1].is_local);
}

#[test]
fn the_stack_is_ordered_with_the_next_resolving_item_first() {
    let bottom = token(50, 0, "Counterspell", 0, 0);
    let top = token(51, 1, "Lightning Bolt", 0, 0);
    let view = ViewBuilder::new(2).with_stack(vec![bottom, top]).build();
    let m = model(&view);
    assert_eq!(m.stack.len(), 2);
    assert_eq!(m.stack[0].name, "Lightning Bolt");
    assert_eq!(m.stack[0].depth, 0);
    assert_eq!(m.stack[1].depth, 1);
}

#[test]
fn the_hand_keeps_the_order_the_cards_arrived_in() {
    // The view's hand is the engine's zone list, which a drawn card is
    // pushed onto — so "the order the view sends" is the order they were
    // drawn, and the model must not touch it. This test used to assert the
    // opposite (playable first, then cheapest); the sort it checked is
    // what moved a card out from under the pointer every time a land
    // untapped.
    let view = ViewBuilder::new(2)
        .with_hand(vec![
            ("Expensive Thing", 7, 100),
            ("Cheap Thing", 1, 101),
            ("Playable Thing", 5, 102),
        ])
        .build();
    let playable: HashSet<ObjectId> = [ObjectId::new(102, 0)].into_iter().collect();
    let m = BoardModel::from_view(
        &view,
        Openings {
            playable: &playable,
            reachable: &HashSet::new(),
            activatable: &HashSet::new(),
        },
        |_| WIDE,
        Registry::none(),
    );
    let names: Vec<&str> = m.hand.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["Expensive Thing", "Cheap Thing", "Playable Thing"],
        "the hand was re-ordered"
    );
    // Playability is still reported — it is now carried by light alone.
    assert!(m.hand[2].playable);
    assert!(!m.hand[0].playable);
}

#[test]
fn one_card_in_three_places_is_one_image() {
    let mut card = token(1, 0, "Serra Angel", 4, 4);
    card.card = Some(CardIdentity {
        index: baylee_core::ids::CardIndex::new(7),
        print: PrintRef::new(3),
        face: 0,
    });
    let mut same = card.clone();
    same.id = ObjectId::new(2, 0);

    let mut on_stack = card.clone();
    on_stack.id = ObjectId::new(3, 0);

    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![card, same])
        .with_stack(vec![on_stack])
        .build();
    let m = model(&view);
    let keys = m.required_images();

    // Three objects, one image: the two battlefield copies group, and the
    // stack copy asks at the same size they do. It used to ask at
    // `Normal`, which made this two entries — and meant a spell cast from
    // a hand the player could already see fetched its art a second time
    // and drew the constructed face until it landed. The board has exactly
    // one size now; the hover preview is the only thing that reads bigger.
    assert_eq!(keys.len(), 1);
    assert!(keys.iter().all(|k| k.size == ArtSize::Small));
}

#[test]
fn tokens_have_no_art_key_and_are_marked_as_tokens() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![token(1, 0, "Soldier", 1, 1)])
        .build();
    let m = model(&view);
    let group = &m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane")
        .groups[0];
    assert_eq!(group.provenance, Provenance::Token);
    assert!(group.art.is_none());
    assert!(m.required_images().is_empty());
}

/// The vanishing land, stated as arithmetic.
///
/// Observed fault 19: the first land played "vanished, reappeared and
/// vanished again — while still being counted". Three observations, one
/// cause. The collapse ran on every board, so a second Forest swallowed
/// the first into a count of two; tapping one for mana split them apart
/// again, because the summary key carries the tap; and untapping put them
/// back together. Nothing was ever miscounted, which is exactly why the
/// count went on being right while the card was not there.
///
/// Both ends of the threshold are pinned, because the collapse still has
/// to happen: it is what forty tokens are for.
#[test]
fn a_second_copy_of_a_land_does_not_swallow_the_first() {
    // A duel's own row. Thirteen cards fit in it at a tap-sized pitch;
    // the fourteenth is where they would have to overlap.
    let roomy = 19.7;
    let forest = |slot: u32| {
        let mut o = printed(slot, 0, "Forest", 7);
        o.types = TypeSet::LAND;
        o.power = None;
        o.toughness = None;
        o
    };
    let two = |tapped: bool| {
        let a = forest(1);
        let mut b = forest(2);
        if tapped {
            b.status = ObjectStatus::TAPPED;
        }
        let view = ViewBuilder::new(2).with_battlefield(0, vec![a, b]).build();
        let m = BoardModel::from_view(&view, Openings::none(), |_| roomy, Registry::none());
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Lands))
            .expect("lane")
            .groups
            .len()
    };
    assert_eq!(two(false), 2, "the second land hid the first");
    // And the number does not change when one of them taps, which is the
    // half the owner actually noticed: a board that rearranges itself
    // every time a land pays for something cannot be read.
    assert_eq!(two(true), 2, "tapping one land redrew the row");

    // The other end. Once the cards would have to overlap, spreading
    // identical ones out shows nothing their count does not, so they
    // become one card again — which is the behaviour forty tokens need
    // and this change must not have thrown away.
    let many = |n: u32| {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, (1..=n).map(forest).collect::<Vec<_>>())
            .build();
        let m = BoardModel::from_view(&view, Openings::none(), |_| roomy, Registry::none());
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Lands))
            .expect("lane");
        (lane.groups.len(), lane.permanent_count())
    };
    assert_eq!(many(13), (13, 13), "a row that fits still draws cards");
    assert_eq!(many(14), (1, 14), "a row that cannot fit still collapses");
}

#[test]
fn a_registry_token_wears_its_own_picture_and_a_copy_token_the_card_it_copies() {
    // Two permanents with no printing between them, drawn from opposite
    // ends. A Soldier the registry knows carries the token id its art is
    // keyed on. A token some clone effect made carries neither that nor a
    // card — a copy of a *card* is on nobody's token list — so the only
    // handle it has ever had is the name it projects, and until the
    // registry was asked about that name it fell back to a coloured
    // rectangle with the name written across it.
    let mut soldier = token(1, 0, "Soldier", 1, 1);
    soldier.token = Some(8);
    let bear_token = token(2, 0, "Bear", 2, 2);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![soldier, bear_token])
        .build();
    let bear = CardIndex::new(7);
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry::of(&|name: &str| (name == "Bear").then_some(Wears::Card(bear, 0))),
    );
    let art = |m: &BoardModel, name: &str| {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .find(|g| g.name == name)
            .expect("group")
            .art
    };
    assert_eq!(
        art(&m, "Soldier"),
        Some(ImageKey::token(8, ArtSize::Small)),
        "a registry token knows which picture it wears"
    );
    assert_eq!(
        art(&m, "Bear"),
        Some(ImageKey::card(bear, 0, ArtSize::Small)),
        "a copy token is drawn as the card it copies"
    );
    for key in [
        ImageKey::token(8, ArtSize::Small),
        ImageKey::card(bear, 0, ArtSize::Small),
    ] {
        assert!(
            m.required_images().contains(&key),
            "{key:?} has to be asked for, or nothing fetches it"
        );
    }

    // The counter-test, and the state every client that has no registry
    // to ask is in: a lookup that answers nothing leaves the copy token
    // exactly where it was — its own face with its own name on it, never
    // somebody else's picture.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
    assert_eq!(art(&blind, "Bear"), None);
    assert_eq!(
        art(&blind, "Soldier"),
        Some(ImageKey::token(8, ArtSize::Small)),
        "and a registry token never needed the lookup"
    );
}

#[test]
fn a_permanent_that_has_become_a_copy_is_drawn_as_the_card_it_copies() {
    // The view says two things at once and only the second is what the
    // player is looking at. `card` is the cardboard — a copy effect
    // assigns characteristics and never a printing (CR 707.2 lists the
    // copiable values; CR 109.3 lists the characteristics, and neither
    // has art in it) — while `name` is the projection. So a Clone that
    // has become a Llanowar Elves was drawn as a Clone with "Llanowar
    // Elves" written under it, which is a card that does not exist.
    //
    // Object 3 is the Clone: its own card index 5, projecting the Elves'
    // name. Object 4 is a real Llanowar Elves, index 9, and it is the
    // control — the registry answers *itself* for it, so the disagreement
    // that marks a copy is absent and it keeps its own printing.
    let clone = printed(3, 0, "Llanowar Elves", 5);
    let real = printed(4, 0, "Llanowar Elves", 9);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![clone, real])
        .build();
    let elves = CardIndex::new(9);
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
    );
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    // Two groups, not one: `ObjectSummaryKey` carries the card, so a
    // Clone wearing another card's face can never be counted into a stack
    // with the card itself — which is what would have hidden the copy the
    // moment the two stood side by side.
    assert_eq!(lane.groups.len(), 2, "the copy and the card it copies");
    let of = |id: u32| {
        lane.groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
            .art
    };
    assert_eq!(
        of(3),
        Some(ImageKey::card(elves, 0, ArtSize::Small)),
        "the copy wears the picture of the card it copies"
    );
    assert_eq!(
        of(4),
        Some(ImageKey::new(PrintRef::new(9), 0, ArtSize::Small)),
        "and the card itself keeps the printing at this table"
    );

    // The counter-test: with no registry to ask, both fall back to their
    // own printings and the Clone is once again drawn as a Clone.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
    let blind_lane = blind
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(
        blind_lane
            .groups
            .iter()
            .find(|g| g.representative == ObjectId::new(3, 0))
            .expect("group")
            .art,
        Some(ImageKey::new(PrintRef::new(5), 0, ArtSize::Small))
    );
}

/// The same disagreement, read as the mark rather than as the picture.
///
/// Four permanents, one of each answer, in one board — because the three
/// arms are a chain and a test that asked them one at a time would not
/// notice an arm swallowing the case below it. The face-down one is the
/// arm that costs something to get right: its `card` is `None` for the
/// same reason a token's is, and reading only that field marks every
/// opponent's morph as a token.
#[test]
fn a_permanent_says_whether_it_is_a_token_a_copy_or_the_card_it_looks_like() {
    let elves = CardIndex::new(9);
    let mut face_down = printed(6, 0, "Face-down", 12);
    face_down.card = None;
    face_down.status = ObjectStatus::FACE_DOWN;
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![
                printed(3, 0, "Llanowar Elves", 5),  // a Clone
                printed(4, 0, "Llanowar Elves", 9),  // the card itself
                token(5, 0, "Llanowar Elves", 1, 1), // a copy token
                face_down,
            ],
        )
        .build();
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
    );
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let of = |id: u32| {
        lane.groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
            .provenance
    };
    assert_eq!(of(3), Provenance::Copy, "cardboard belonging to a Clone");
    assert_eq!(of(4), Provenance::Printed, "the card it looks like");
    // A token a copy effect made is a token and not a copy: there is no
    // original to go and look at, so a copy mark would promise one.
    assert_eq!(of(5), Provenance::Token, "a chit, whatever is drawn on it");
    assert_eq!(
        of(6),
        Provenance::Printed,
        "a morph is not a token, however alike the two look in the view"
    );

    // The counter-test. A client with nothing to ask still knows a token
    // from a card — that half needs no registry — and simply never finds
    // a copy, which is what it did before any of this existed.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
    let blind_lane = blind
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let blind_of = |id: u32| {
        blind_lane
            .groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
            .provenance
    };
    assert_eq!(blind_of(3), Provenance::Printed);
    assert_eq!(blind_of(5), Provenance::Token);
}

/// The card underneath a copy is offered beside the one it is wearing.
///
/// Two claims, and the second is the one that would rot quietly: the key
/// is the *physical* printing rather than the worn card, and the board
/// asks for it to be resident — a preview opens on a hover and has no
/// frame to spend on a fetch.
#[test]
fn a_copy_offers_the_card_underneath_it() {
    let elves = CardIndex::new(9);
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![
                printed(3, 0, "Llanowar Elves", 5),  // a Clone
                printed(4, 0, "Llanowar Elves", 9),  // the card itself
                token(5, 0, "Llanowar Elves", 1, 1), // a copy token
            ],
        )
        .build();
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
    );
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let group = |id: u32| {
        lane.groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
    };
    let cardboard = ImageKey::new(PrintRef::new(5), 0, ArtSize::Small);
    assert_eq!(group(3).original, Some(cardboard), "the Clone's own print");
    assert_eq!(group(4).original, None, "a card stands in for nothing");
    assert_eq!(group(5).original, None, "a chit has nothing underneath it");
    // A second key beside the first and never a replacement for it: what
    // the table draws is still the card the copy is wearing.
    assert_eq!(
        group(3).art,
        Some(ImageKey::card(elves, 0, ArtSize::Small)),
        "the copy is still drawn as what it copies"
    );
    assert!(
        m.required_images().contains(&cardboard),
        "the hover would have to fetch it"
    );

    // The counter-arm. A client with nothing to ask finds no copies, so
    // no card on its board carries a second picture at all.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
    assert!(
        blind
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .all(|g| g.original.is_none())
    );
}

/// A permanent copying a *token* is a copy, and is drawn as the chit.
///
/// The case entry 16 of `docs/observed-faults.md` was still getting
/// wrong. Both the mark and the picture were found by handing the
/// projected name to a lookup that only knew cards, so a Clone on a
/// Soldier answered the same `None` a permanent copying nothing answers:
/// no mark, no card underneath, and a Clone on the table with "Soldier"
/// written under it.
#[test]
fn a_permanent_copying_a_token_wears_the_token() {
    const SOLDIER: u16 = 9;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(3, 0, "Soldier", 5)])
        .build();
    let named = |name: &str| (name == "Soldier").then_some(Wears::Token(SOLDIER));
    let token_name = |id: u16| (id == SOLDIER).then_some("Soldier");
    let group = |m: &BoardModel| {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups[0]
            .clone()
    };

    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry {
            named: &named,
            token_name: &token_name,
        },
    );
    let g = group(&m);
    assert_eq!(g.provenance, Provenance::Copy, "it is copying something");
    assert_eq!(
        g.art,
        Some(ImageKey::token(SOLDIER, ArtSize::Small)),
        "and what it is copying is a chit, which has its own picture"
    );
    let cardboard = ImageKey::new(PrintRef::new(5), 0, ArtSize::Small);
    assert_eq!(g.original, Some(cardboard), "the Clone is still underneath");
    assert!(m.required_images().contains(&cardboard));

    // The counter-arm, and it is the bug this test is about: a registry
    // that cannot name the token draws the Clone as itself and says
    // nothing is unusual.
    let blind = group(&BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry::none(),
    ));
    assert_eq!(blind.provenance, Provenance::Printed);
    assert_eq!(blind.art, Some(cardboard));
    assert_eq!(blind.original, None);
}

/// A token is never a copy of its own twin.
///
/// Two tokens in the registry are printed "Shapeshifter", so a name
/// answers with the first of them and an identity test made of names
/// would call the second a copy of the first — and draw the 2/2 as the
/// 1/1. A token's own name is asked for by *id* for exactly this reason,
/// which is the one asymmetry between the two arms of [`worn`].
#[test]
fn a_token_is_not_a_copy_of_the_twin_that_shares_its_name() {
    const ONE_ONE: u16 = 6;
    const TWO_TWO: u16 = 7;
    let mut chit = token(3, 0, "Shapeshifter", 2, 2);
    chit.token = Some(TWO_TWO);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![chit, printed(4, 0, "Shapeshifter", 5)])
        .build();
    let named = |name: &str| (name == "Shapeshifter").then_some(Wears::Token(ONE_ONE));
    let token_name = |id: u16| (id == ONE_ONE || id == TWO_TWO).then_some("Shapeshifter");
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        Registry {
            named: &named,
            token_name: &token_name,
        },
    );
    let group = |id: u32| {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
    };
    assert_eq!(group(3).provenance, Provenance::Token);
    assert_eq!(
        group(3).art,
        Some(ImageKey::token(TWO_TWO, ArtSize::Small)),
        "the 2/2 is drawn from its own id and not from its name"
    );
    assert_eq!(group(3).original, None, "a chit has nothing underneath it");

    // And the tie the other way round, which is the documented one: a
    // *card* copying either Shapeshifter is drawn as the first of them,
    // because a projected name is all there is to go on and two tokens
    // with one name are one name.
    assert_eq!(group(4).provenance, Provenance::Copy);
    assert_eq!(group(4).art, Some(ImageKey::token(ONE_ONE, ArtSize::Small)));
}

#[test]
fn each_pod_is_measured_against_its_own_row() {
    // Seats do not get equal space, so the collapse cannot be decided by
    // one width for the whole table: the same four Soldiers are four
    // cards on a roomy pod and one counted card on a cramped one, in the
    // *same* board. Before this, the width was read off the first
    // opponent and every other seat — the local one included — was gated
    // against a row it was not standing on.
    let squad = |seat: u8| -> Vec<PublicObject> {
        (0..4)
            .map(|i| token(u32::from(seat) * 10 + i, seat, "Soldier", 1, 1))
            .collect()
    };
    let view = ViewBuilder::new(2)
        .with_battlefield(0, squad(0))
        .with_battlefield(1, squad(1))
        .build();
    let groups = |m: &BoardModel, seat: u8| {
        m.pod(PlayerId::new(seat))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .len()
    };

    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |p| {
            if p == PlayerId::new(0) { WIDE } else { CROWDED }
        },
        Registry::none(),
    );
    assert_eq!(groups(&m, 0), 4, "the roomy pod kept its cards apart");
    assert_eq!(groups(&m, 1), 1, "the cramped pod collapsed its own row");

    // The counter-test: the widths are what decide it, not the seat.
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |p| {
            if p == PlayerId::new(0) { CROWDED } else { WIDE }
        },
        Registry::none(),
    );
    assert_eq!(groups(&m, 0), 1);
    assert_eq!(groups(&m, 1), 4);
}

#[test]
fn a_narrow_pod_reports_overflow_after_grouping() {
    // Forty *distinct* permanents cannot be grouped, so a small pod has to
    // scroll rather than fan.
    let objs: Vec<PublicObject> = (0..40)
        .map(|i| token(i, 0, &format!("Creature {i}"), 1, 1))
        .collect();
    let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
    let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0, Registry::none());
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(lane.groups.len(), 40);
    assert!(lane.overflowing);
}

#[test]
fn grouping_removes_the_overflow_that_distinct_cards_would_cause() {
    // The same forty permanents, all identical: one group, no overflow.
    let objs: Vec<PublicObject> = (0..40).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
    let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0, Registry::none());
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(lane.groups.len(), 1);
    assert!(!lane.overflowing);
}

#[test]
fn a_board_resolves_end_to_end_into_fetchable_image_urls() {
    use crate::images::resolve;
    use crate::test_support::{printed, statics};

    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Serra Angel", 4)])
        .build();
    let m = model(&view);
    let table = statics(8);

    let keys = m.required_images();
    assert_eq!(keys.len(), 1);
    let request =
        resolve(&table, keys[0], |_| None, |_| None).expect("the print table resolves it");
    assert!(
        request
            .url
            .starts_with("https://cards.scryfall.io/small/front/")
    );
    assert!(
        std::path::Path::new(&request.url)
            .extension()
            .is_some_and(|e| e == "jpg")
    );
}

#[test]
fn priority_is_reported_on_the_seat_that_holds_it() {
    let view = ViewBuilder::new(3).with_priority(Some(2)).build();
    let m = model(&view);
    assert!(!m.pod(PlayerId::new(0)).expect("pod").has_priority);
    assert!(m.pod(PlayerId::new(2)).expect("pod").has_priority);

    let nobody = ViewBuilder::new(3).with_priority(None).build();
    let m = model(&nobody);
    assert!(m.pods.iter().all(|p| !p.has_priority));
}

/// A spell on the stack and the permanent it is pointed at.
fn bolt_at_bears() -> PlayerView {
    let bears = printed(1, 1, "Grizzly Bears", 11);
    let mut bolt = printed(2, 0, "Lightning Bolt", 22);
    bolt.types = TypeSet::INSTANT;
    bolt.power = None;
    bolt.toughness = None;
    bolt.stack_item = Some(baylee_view::StackItem::Spell);
    bolt.targets = vec![TargetRef::Object(ObjectId::new(1, 0))];
    ViewBuilder::new(2)
        .with_battlefield(1, [bears])
        .with_stack(vec![bolt])
        .build()
}

#[test]
fn a_spell_on_the_stack_says_what_it_points_at() {
    let view = bolt_at_bears();
    let m = model(&view);
    let item = &m.stack[0];
    assert_eq!(item.kind, StackKind::Spell);
    assert!(item.art.is_some(), "a spell shows its own card");
    assert_eq!(item.targets.len(), 1);
    // The whole point: a handle is not drawable, a name and a face are.
    assert_eq!(item.targets[0].name.as_deref(), Some("Grizzly Bears"));
    assert!(item.targets[0].art.is_some());
    assert_eq!(item.targets[0].object(), Some(ObjectId::new(1, 0)));
}

#[test]
fn a_targets_picture_is_kept_resident_too() {
    let view = bolt_at_bears();
    let m = model(&view);
    let art = m.stack[0].targets[0].art.expect("the target has a face");
    assert!(
        m.required_images().contains(&art),
        "a target drawn beside the spell has to be loaded like anything else"
    );
}

#[test]
fn an_ability_on_the_stack_borrows_its_sources_picture() {
    let source = printed(1, 0, "Llanowar Elves", 33);
    let source_art = ImageKey::new(PrintRef::new(33), 0, ArtSize::Small);
    let mut ability = token(2, 0, "Llanowar Elves", 0, 0);
    ability.card = None;
    ability.types = TypeSet::EMPTY;
    ability.power = None;
    ability.toughness = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        source: ObjectId::new(1, 0),
        ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(33), 0)),
        text: None,
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [source])
        .with_stack(vec![ability])
        .build();

    let m = model(&view);
    assert_eq!(
        m.stack[0].kind,
        StackKind::Ability {
            source: ObjectId::new(1, 0),
            text: None,
        }
    );
    assert_eq!(
        m.stack[0].art,
        Some(source_art),
        "an ability has no card, so it wears the picture of whatever made it"
    );
}

/// The picture and the sentence are two halves of one card, so the face
/// the host named for the *text* is the face the borrowed picture is
/// taken from — not the face the source happens to be showing.
///
/// A Sheoldred who has turned back over while her chapter ability waits
/// on the stack is the shape of it (CR 113.7a): the permanent on the
/// battlefield is face 0, and the ability is face 1's third sentence.
/// Drawing face 0 beside face 1's text would be one card illustrated
/// with another.
#[test]
fn an_abilitys_picture_is_taken_from_the_face_its_text_came_from() {
    let source = printed(1, 0, "Sheoldred", 33);
    let mut ability = token(2, 0, "Sheoldred", 0, 0);
    ability.card = None;
    ability.types = TypeSet::EMPTY;
    ability.power = None;
    ability.toughness = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        source: ObjectId::new(1, 0),
        ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(33), 2)),
        text: Some(baylee_view::StackText {
            face: 1,
            line: 2,
            of: 3,
        }),
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [source])
        .with_stack(vec![ability])
        .build();

    let m = model(&view);
    assert_eq!(
        m.stack[0].art,
        Some(ImageKey::new(PrintRef::new(33), 1, ArtSize::Small)),
        "the source shows face 0 and the ability came off face 1"
    );
    assert!(
        m.required_images()
            .contains(&ImageKey::new(PrintRef::new(33), 1, ArtSize::Small)),
        "the face actually drawn is the face that has to be loaded"
    );
}

#[test]
fn an_ability_whose_source_is_gone_still_draws() {
    let mut ability = token(2, 0, "Cast Down", 0, 0);
    ability.card = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        source: ObjectId::new(99, 0),
        ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(1), 0)),
        text: None,
    });
    let view = ViewBuilder::new(2).with_stack(vec![ability]).build();
    let m = model(&view);
    // CR 113.7a: the ability is independent of its source. No picture to
    // borrow is a missing picture, never a missing entry.
    assert_eq!(m.stack[0].art, None);
    assert_eq!(m.stack[0].name, "Cast Down");
}

#[test]
fn a_targeted_player_has_no_card_to_draw() {
    let mut bolt = printed(2, 0, "Lightning Bolt", 22);
    bolt.stack_item = Some(baylee_view::StackItem::Spell);
    bolt.targets = vec![TargetRef::Player(PlayerId::new(1))];
    let view = ViewBuilder::new(2).with_stack(vec![bolt]).build();
    let m = model(&view);
    let target = &m.stack[0].targets[0];
    assert_eq!(target.player(), Some(PlayerId::new(1)));
    assert_eq!(target.object(), None);
    // The seat's name lives in `GameStatic`, which this model has never
    // carried — so the renderer, not the model, spells a player out.
    assert_eq!(target.name, None);
    assert_eq!(target.art, None);
}

#[test]
fn a_group_is_activatable_only_when_every_card_in_it_is() {
    let objs = vec![token(1, 0, "Forest", 0, 0), token(2, 0, "Forest", 0, 0)];
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();

    let both: HashSet<ObjectId> = [ObjectId::new(1, 0), ObjectId::new(2, 0)]
        .into_iter()
        .collect();
    let one: HashSet<ObjectId> = std::iter::once(ObjectId::new(1, 0)).collect();
    let empty = HashSet::new();

    let openings = |set| Openings {
        playable: &empty,
        reachable: &empty,
        activatable: set,
    };

    let lit = BoardModel::from_view(&view, openings(&both), |_| CROWDED, Registry::none());
    let group = &lit.pods[0].lanes[0].groups[0];
    assert_eq!(group.count(), 2, "identical permanents still merge");
    assert!(group.activatable);

    // One of the two cannot be tapped, so the card standing for both must
    // not claim it can — the player would click it and be told no.
    let half = BoardModel::from_view(&view, openings(&one), |_| CROWDED, Registry::none());
    assert!(!half.pods[0].lanes[0].groups[0].activatable);

    let dark = BoardModel::from_view(&view, openings(&empty), |_| CROWDED, Registry::none());
    assert!(!dark.pods[0].lanes[0].groups[0].activatable);
}

#[test]
fn the_model_is_deterministic_for_a_given_view() {
    let objs: Vec<PublicObject> = (0..20)
        .map(|i| token(i, 0, if i % 2 == 0 { "A" } else { "B" }, 1, 1))
        .collect();
    let view = ViewBuilder::new(4).with_battlefield(0, objs).build();
    let a = model(&view);
    let b = model(&view);
    assert_eq!(a, b, "the same view must always produce the same scene");
}
