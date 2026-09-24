//! One row of a seat's board: which lane a permanent stands in, when identical permanents collapse into one counted card, and when a row has to scroll instead. Collapsing may shorten a board but must never change what a player would conclude from it, so both ends of the threshold are pinned — a row that still fits draws its cards, a row that cannot fit collapses, and each pod is measured against its own width and not the table's — together with every reason a permanent stays its own card however alike it looks: tapped, attacking, blocked, blocking, attached, enchanted, targeted. A permanent that is on no row at all, phased out, is here for the same reason. What a card is *drawn as* is `provenance`, and what it claims a player may do with it is `openings`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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

/// Tokens merge on any row, cards only on a full one (#210). A spell that
/// leaves three Soldiers leaves one card saying three, however much room
/// the row has; tapping one splits it off, because whether a blocker is
/// still up is what a player reads the row for.
#[test]
fn tokens_merge_on_a_roomy_row_and_split_by_state() {
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

    assert_eq!(
        lane_of(soldiers()),
        vec![3],
        "three Soldiers on a roomy row"
    );

    let mut one_tapped = soldiers();
    one_tapped[1].status = ObjectStatus::TAPPED;
    assert_eq!(
        lane_of(one_tapped),
        vec![1, 2],
        "the tapped one stands apart"
    );

    // The counter-test: the same three as cards keep the room test.
    let bears: Vec<PublicObject> = (0..3).map(|i| printed(i, 0, "Grizzly Bears", 7)).collect();
    assert_eq!(
        lane_of(bears),
        vec![1, 1, 1],
        "cards on a roomy row stay cards"
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
                defending: Defender::Player(PlayerId::new(1)),
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
fn phased_out_permanents_leave_the_board_entirely() {
    let mut objs: Vec<PublicObject> = (0..3).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
    objs[0].status = ObjectStatus::from_bits(ObjectStatus::PHASED_OUT.bits());
    let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
    let m = model(&view);
    assert_eq!(m.pod(PlayerId::new(0)).expect("pod").permanent_count(), 2);
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
        let m = BoardModel::from_view(&view, Openings::none(), |_| roomy, &[], Registry::none());
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
        let m = BoardModel::from_view(&view, Openings::none(), |_| roomy, &[], Registry::none());
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
fn each_pod_is_measured_against_its_own_row() {
    // Seats do not get equal space, so the collapse cannot be decided by
    // one width for the whole table: the same four Bears are four cards
    // on a roomy pod and one counted card on a cramped one, in the *same*
    // board. Before this, the width was read off the first opponent and
    // every other seat — the local one included — was gated against a
    // row it was not standing on. Cards, because tokens merge on any row.
    let squad = |seat: u8| -> Vec<PublicObject> {
        (0..4)
            .map(|i| printed(u32::from(seat) * 10 + i, seat, "Grizzly Bears", 7))
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
        &[],
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
        &[],
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
    let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0, &[], Registry::none());
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
    let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0, &[], Registry::none());
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(lane.groups.len(), 1);
    assert!(!lane.overflowing);
}
