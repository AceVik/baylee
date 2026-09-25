//! One row of a seat's board: which lane a permanent stands in, which section of it (#263), when identical permanents pile into one counted card, and when a row has to scroll instead. Piling may shorten a board but must never change what a player would conclude from it, so every difference a player reads splits a pile — tapped, summoning sick, a counter, damage — and every reason a permanent stays its own card however alike it looks is pinned: attacking, blocked, blocking, attached, enchanted, targeted, face down. A permanent that is on no row at all, phased out, is here for the same reason. What a card is *drawn as* is `provenance`, and what it claims a player may do with it is `openings`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn identical_tokens_collapse_into_one_counted_card() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, (0..12).map(|i| token(i, 0, "Soldier", 1, 1)))
        .build();
    let m = model(&view);
    let pod = m.pod(PlayerId::new(0)).expect("pod");
    let lane = pod.lane(LaneKind::Creatures).expect("creature lane");

    assert_eq!(lane.groups.len(), 1, "twelve identical tokens draw as one");
    assert_eq!(lane.groups[0].count(), 12);
    assert!(lane.groups[0].is_stack());
    assert_eq!(lane.permanent_count(), 12);
}

/// Identical permanents pile on any row, tokens (#210) and cards alike (the
/// owner, 25.09), and a pile splits by state: tapping one splits it off,
/// because whether a blocker is still up is what a player reads the row for.
#[test]
fn identical_permanents_pile_on_any_row_and_split_by_state() {
    let lane_of = |objs: Vec<PublicObject>| {
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = model(&view);
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        let mut counts: Vec<usize> = lane.groups.iter().map(CardGroup::count).collect();
        counts.sort_unstable();
        counts
    };
    let soldiers =
        || -> Vec<PublicObject> { (0..3).map(|i| token(i, 0, "Soldier", 1, 1)).collect() };

    assert_eq!(lane_of(soldiers()), vec![3], "three Soldiers");

    let mut one_tapped = soldiers();
    one_tapped[1].status = ObjectStatus::TAPPED;
    assert_eq!(
        lane_of(one_tapped),
        vec![1, 2],
        "the tapped one stands apart"
    );

    let bears: Vec<PublicObject> = (0..3).map(|i| printed(i, 0, "Grizzly Bears", 7)).collect();
    assert_eq!(lane_of(bears), vec![3], "three Bears on a roomy row");
}

/// A pile never hides a difference one of its cards has and another has
/// not (the owner, 25.09): a Soldier given a +1/+1 counter leaves the plain
/// Soldiers' pile, Soldiers that each carry one make a pile of their own,
/// and the same goes for damage and for a different body. Checked on every
/// pile against every member, so a key that forgot a field fails here
/// whichever field it was.
#[test]
fn a_counter_is_never_merged_away() {
    use baylee_view::{CounterEntry, CounterKind};
    let plus = |count: u16| {
        vec![CounterEntry {
            kind: CounterKind::PLUS_ONE,
            count,
        }]
    };
    let mut objs: Vec<PublicObject> = (0..9).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    for o in &mut objs[3..5] {
        o.counters = plus(1);
    }
    objs[5].counters = plus(2);
    objs[6].damage = 1;
    objs[7].power = Some(3);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, objs.clone())
        .build();
    let m = model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let of = |id: ObjectId| objs.iter().find(|o| o.id == id).expect("an object");
    let state = |o: &PublicObject| (o.counters.clone(), o.damage, o.power, o.toughness, o.status);
    for group in &lane.groups {
        let top = state(of(group.representative));
        for &member in &group.members {
            assert_eq!(
                state(of(member)),
                top,
                "{member:?} is piled under {:?} with a different state",
                group.representative
            );
        }
    }
    let mut counts: Vec<usize> = lane.groups.iter().map(CardGroup::count).collect();
    counts.sort_unstable();
    assert_eq!(
        counts,
        vec![1, 1, 1, 2, 4],
        "plain, one counter, two, hurt, grown"
    );
}

/// Summoning sick, tapped and neither are three piles (the owner, 25.09):
/// a creature that cannot attack yet does not hide among ones that can.
#[test]
fn sick_tapped_and_ready_are_three_piles() {
    let mut objs: Vec<PublicObject> = (0..6).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[0].summoning_sick = true;
    objs[1].summoning_sick = true;
    objs[2].status = ObjectStatus::TAPPED;
    objs[3].status = ObjectStatus::TAPPED;
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let counts: Vec<usize> = lane.groups.iter().map(CardGroup::count).collect();
    assert_eq!(counts, vec![2, 2, 2]);
}

/// What the answer being built proposes splits a stack the way a sent
/// declaration does (#210), so the card a player clicked never says more is
/// declared than is: three Soldiers sent at a seat are a `×3` beside a `×9`,
/// two sent at a planeswalker a card of their own again, and targets picked
/// out of a stack likewise.
#[test]
fn a_proposal_splits_a_stack_by_what_it_proposes() {
    let soldiers: Vec<PublicObject> = (0..12).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    let ids: Vec<ObjectId> = soldiers.iter().map(|o| o.id).collect();
    let view = ViewBuilder::new(2).with_battlefield(0, soldiers).build();
    let drawn = |proposed: &[(usize, Proposal)]| {
        let proposed: HashMap<ObjectId, Proposal> =
            proposed.iter().map(|(at, p)| (ids[*at], *p)).collect();
        let m = BoardModel::from_view(
            &view,
            Openings {
                proposed: &proposed,
                ..Openings::none()
            },
            &[],
            Registry::none(),
        );
        let mut cards: Vec<(usize, Option<Proposal>)> = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .map(|g| (g.count(), g.proposed))
            .collect();
        cards.sort_by_key(|(count, _)| *count);
        cards
    };
    let at = |defender| Proposal::Combat(CombatFocus::Defender(defender));
    let seat = at(Defender::Player(PlayerId::new(1)));
    let walker = at(Defender::Planeswalker(ObjectId::new(99, 0)));

    assert_eq!(drawn(&[]), vec![(12, None)]);
    assert_eq!(
        drawn(&[(9, seat), (10, seat), (11, seat)]),
        vec![(3, Some(seat)), (9, None)]
    );
    assert_eq!(
        drawn(&[(10, walker), (11, walker), (9, seat)]),
        vec![(1, Some(seat)), (2, Some(walker)), (9, None)]
    );
    assert_eq!(
        drawn(&[(0, Proposal::Picked), (1, Proposal::Picked)]),
        vec![(2, Some(Proposal::Picked)), (10, None)]
    );
}

/// A card is found by the object it is drawn as, and only by that one: the
/// other members have no card, so a pointer is never on them.
#[test]
fn a_group_is_found_by_its_representative_and_by_nothing_else() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, (0..3).map(|i| token(i, 0, "Soldier", 1, 1)))
        .build();
    let m = model(&view);
    let drawn = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane")
        .groups[0]
        .clone();

    let found = m.group(drawn.representative).expect("the drawn card");
    assert_eq!(found.count(), 3);
    let other = drawn
        .members
        .iter()
        .find(|m| **m != drawn.representative)
        .expect("a second member");
    assert!(
        m.group(*other).is_none(),
        "a member without a card was found"
    );
}

#[test]
fn a_tapped_token_does_not_hide_inside_the_untapped_stack() {
    let mut objs: Vec<PublicObject> = (0..5).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[3].status = ObjectStatus::TAPPED;
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
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
                defending: Defender::Player(PlayerId::new(1)),
                blocked: true,
            }],
            vec![BlockerView { blocker, attacker }],
        )
        .build();
    let m = model(&view);
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
    let m = model(&view);
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
    let m = model(&view);
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
fn phased_out_permanents_leave_the_board_entirely() {
    let mut objs: Vec<PublicObject> = (0..3).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[0].status = ObjectStatus::from_bits(ObjectStatus::PHASED_OUT.bits());
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    assert_eq!(m.pod(PlayerId::new(0)).expect("pod").permanent_count(), 2);
}

/// The vanishing land, as the owner now asks for it.
///
/// Observed fault 19 was a second Forest swallowing the first into a count
/// of two, tapping one splitting them apart and untapping putting them back:
/// nothing miscounted, and a card that was not there. Piles on every row
/// (the owner, 25.09) make that the rule rather than the fault, and the
/// answer to "the card was not there" is that the pile shows it is: two
/// Forests are one card with a second under it and a `×2`, and a tapped one
/// stands beside the untapped as a pile of its own.
#[test]
fn identical_lands_pile_and_a_tapped_one_stands_beside_them() {
    let forest = |slot: u32| {
        let mut o = printed(slot, 0, "Forest", 7);
        o.types = TypeSet::LAND;
        o.power = None;
        o.toughness = None;
        o
    };
    let lane = |objs: Vec<PublicObject>| {
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = model(&view);
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Lands))
            .expect("lane");
        let mut counts: Vec<usize> = lane.groups.iter().map(CardGroup::count).collect();
        counts.sort_unstable();
        counts
    };
    assert_eq!(
        lane(vec![forest(1), forest(2)]),
        vec![2],
        "two Forests pile"
    );
    let mut tapped = forest(2);
    tapped.status = ObjectStatus::TAPPED;
    assert_eq!(
        lane(vec![forest(1), tapped]),
        vec![1, 1],
        "a tapped Forest stands beside the untapped one"
    );
    assert_eq!(lane((1..=13).map(forest).collect()), vec![13]);
}

#[test]
fn a_narrow_pod_reports_overflow_after_grouping() {
    // Forty *distinct* permanents cannot be grouped, so a small pod has to
    // scroll rather than fan.
    let objs: Vec<PublicObject> = (0..40)
        .map(|i| token(i, 0, &format!("Creature {i}"), 1, 1))
        .collect();
    let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
    let m = model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(lane.groups.len(), 40);
    assert!(
        lane.pack_at(5.0, BadgePlace::Beside).overflowing,
        "forty distinct creatures fit a small pod"
    );
}

#[test]
fn grouping_removes_the_overflow_that_distinct_cards_would_cause() {
    // The same forty permanents, all identical: one group, no overflow.
    let objs: Vec<PublicObject> = (0..40).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
    let m = model(&view);
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(lane.groups.len(), 1);
    assert!(!lane.pack_at(5.0, BadgePlace::Beside).overflowing);
}

/// Only a creature is modelled asleep, whatever a host says (CR 302.6). A
/// land played this turn taps perfectly well, and a board where every fresh
/// permanent wore the moon would be teaching a player something false. The
/// view carries the narrower fact today; this is what holds if it ever
/// carries the wider one again.
#[test]
fn only_a_creature_is_modelled_asleep() {
    let mut bear = token(1, 0, "Bear", 2, 2);
    bear.summoning_sick = true;
    let mut land = token(2, 0, "Forest", 0, 0);
    land.types = TypeSet::LAND;
    land.power = None;
    land.toughness = None;
    land.summoning_sick = true;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [bear, land])
        .build();
    let m = model(&view);
    let pod = m.pod(PlayerId::new(0)).expect("pod");
    let asleep = |kind| pod.lane(kind).expect("the lane").groups[0].summoning_sick;
    assert!(asleep(LaneKind::Creatures), "the creature sleeps");
    assert!(
        !asleep(LaneKind::Lands),
        "a land that arrived this turn does not"
    );
}

/// `o` as a permanent of `types` with no power or toughness.
fn typed(mut o: PublicObject, types: TypeSet) -> PublicObject {
    o.types = types;
    o.power = None;
    o.toughness = None;
    o
}

/// Every row stands in sections (#263): the many on the left, the ordinary
/// in the centre, the singular and the used on the right, and within the
/// land row the basics in WUBRG order. A row of one kind is one section,
/// which the packing centres.
#[test]
fn each_row_stands_in_sections() {
    use baylee_core::generated::subtypes::land;
    use baylee_core::types::{SubtypeSet, SupertypeSet};
    const TOWER: u16 = 40;
    let land_card =
        |slot: u32, name: &str, print: u16, basic: Option<baylee_core::ids::SubtypeId>| {
            let mut o = typed(printed(slot, 0, name, print), TypeSet::LAND);
            if let Some(kind) = basic {
                o.supertypes = SupertypeSet::BASIC;
                o.subtypes = SubtypeSet::from_slice(&[kind]);
            }
            o
        };
    let legend = {
        let mut o = printed(20, 0, "Aragorn", 21);
        o.supertypes = SupertypeSet::LEGENDARY;
        o
    };
    let commander = {
        let mut o = printed(22, 0, "Atraxa", 23);
        o.supertypes = SupertypeSet::LEGENDARY;
        o.commander = true;
        o
    };
    let objs = vec![
        land_card(1, "Reliquary Tower", TOWER, None),
        land_card(2, "Forest", 3, Some(land::FOREST)),
        land_card(4, "Tundra", 5, None),
        land_card(6, "Plains", 7, Some(land::PLAINS)),
        commander,
        legend,
        printed(30, 0, "Grizzly Bears", 31),
        token(32, 0, "Soldier", 1, 1),
        typed(printed(24, 0, "Jace", 25), TypeSet::PLANESWALKER),
        typed(printed(26, 0, "Sol Ring", 27), TypeSet::ARTIFACT),
        typed(token(28, 0, "Treasure", 0, 0), TypeSet::ARTIFACT),
    ];
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let utility = |face: RulesFace| face.card == CardIndex::new(u32::from(TOWER));
    let reg = Registry {
        utility_land: &utility,
        ..Registry::none()
    };
    let m = BoardModel::from_view(&view, Openings::none(), &[], reg);
    let row = |kind: LaneKind| -> Vec<(String, Section)> {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(kind))
            .expect("lane")
            .groups
            .iter()
            .map(|g| (g.name.clone(), g.section))
            .collect()
    };
    let named = |pairs: &[(&str, Section)]| -> Vec<(String, Section)> {
        pairs.iter().map(|(n, s)| ((*n).to_owned(), *s)).collect()
    };
    assert_eq!(
        row(LaneKind::Lands),
        named(&[
            ("Plains", Section::Left),
            ("Forest", Section::Left),
            ("Tundra", Section::Centre),
            ("Reliquary Tower", Section::Right),
        ])
    );
    assert_eq!(
        row(LaneKind::Creatures),
        named(&[
            ("Soldier", Section::Left),
            ("Grizzly Bears", Section::Centre),
            ("Aragorn", Section::Right),
            ("Atraxa", Section::Right),
        ])
    );
    assert_eq!(
        row(LaneKind::Support),
        named(&[
            ("Treasure", Section::Left),
            ("Sol Ring", Section::Centre),
            ("Jace", Section::Right),
        ])
    );
    let lands = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Lands))
        .expect("lane");
    assert_eq!(
        lands.gaps(BadgePlace::Above),
        vec![Gap::Free, Gap::Section, Gap::Section, Gap::Free],
        "the land row's three sections stand apart"
    );
}

/// A card keeps its section whatever it is doing (#263): tapped, sick or
/// attacking, a legend stays on the right and a token on the left, so a row
/// never reshuffles because a turn went by.
#[test]
fn a_card_keeps_its_section_whatever_it_does() {
    use baylee_core::types::SupertypeSet;
    let board = |tapped: bool, sick: bool| {
        let mut legend = printed(1, 0, "Aragorn", 2);
        legend.supertypes = SupertypeSet::LEGENDARY;
        let mut soldier = token(3, 0, "Soldier", 1, 1);
        let bears = printed(4, 0, "Grizzly Bears", 5);
        for o in [&mut legend, &mut soldier] {
            if tapped {
                o.status = ObjectStatus::TAPPED;
            }
            o.summoning_sick = sick;
        }
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![legend, soldier, bears])
            .build();
        model(&view)
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .map(|g| (g.name.clone(), g.section))
            .collect::<Vec<_>>()
    };
    let rest = board(false, false);
    assert_eq!(board(true, false), rest, "tapping moved a card");
    assert_eq!(board(false, true), rest, "sickness moved a card");
}
