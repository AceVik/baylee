use super::*;

/// The two mana fields are what a client plans a turn with, and they are
/// carried per ability rather than per permanent: a permanent may have
/// several, and the one that makes mana is not always the first. Reading
/// the colours onto the wrong row is a land the planner counts and the
/// engine refuses.
#[test]
fn a_mana_row_names_the_ability_it_belongs_to() {
    use baylee_core::mana::ManaColor;
    let mut o = obj(1, 0);
    o.granted_mana = Some(GrantedMana {
        slot: 2,
        colors: vec![ManaColor::White, ManaColor::Blue],
        amount: 1,
    });
    o.board_mana = Some(BoardMana {
        index: 1,
        colors: vec![ManaColor::Green],
    });

    let text = serde_json::to_string(&o).expect("an object serialises");
    let back: PublicObject = serde_json::from_str(&text).expect("and reads back");
    assert_eq!(back.granted_mana, o.granted_mana);
    assert_eq!(back.board_mana, o.board_mana);

    let granted = back.granted_mana.expect("carried");
    assert_eq!(granted.slot, 2, "not the first granted ability");
    assert_eq!(
        granted.colors.len(),
        2,
        "more than one colour is an ability that asks"
    );
    assert_eq!(
        back.board_mana.expect("carried").index,
        1,
        "a printed mana ability that is not the card's first — \
         Commander's Sphere prints a sacrifice ability beside it"
    );
}

/// An object nobody granted anything carries neither field, which is what
/// makes the presence of one meaningful: an ability that can make no mana
/// right now is reported as no row at all rather than as an empty one.
#[test]
fn an_ordinary_permanent_carries_no_mana_row() {
    let o = obj(1, 0);
    assert!(o.granted_mana.is_none() && o.board_mana.is_none());
}

/// Counters are read by kind, and a kind that is not there is nought
/// rather than missing — a client draws the badge from this number and
/// would otherwise have to tell an absent counter from a zero one.
#[test]
fn a_counter_that_is_not_there_counts_as_none() {
    let mut o = obj(1, 0);
    assert_eq!(o.counter_count(CounterKind::PLUS_ONE), 0);
    o.counters = vec![
        CounterEntry {
            kind: CounterKind::PLUS_ONE,
            count: 3,
        },
        CounterEntry {
            kind: CounterKind::Charge,
            count: 1,
        },
    ];
    assert_eq!(o.counter_count(CounterKind::PLUS_ONE), 3);
    assert_eq!(o.counter_count(CounterKind::Charge), 1);
    assert_eq!(o.counter_count(CounterKind::Loyalty), 0, "still nought");
}
/// A distinct number per counter kind, and the guard the golden badges
/// below need: [`CounterKind::badge`] ends in no catch-all, but the two
/// constant patterns at the top of it mean the table is read in order,
/// so a kind added between them would be answered by the arm underneath
/// rather than by one of its own. This match is exhaustive and names
/// each kind once.
fn counter_index(kind: CounterKind) -> usize {
    match kind {
        CounterKind::Plus { .. } => 0,
        CounterKind::Minus { .. } => 1,
        CounterKind::Loyalty => 2,
        CounterKind::Lore => 3,
        CounterKind::Time => 4,
        CounterKind::Charge => 5,
        CounterKind::Poison => 6,
        CounterKind::Energy => 7,
        CounterKind::Rad => 8,
        CounterKind::Lifelink => 9,
        CounterKind::Level => 10,
        CounterKind::Custom(_) => 11,
    }
}

/// Every counter's badge, and which two of them are free.
///
/// The badge is what a client prints on a permanent, so a kind that
/// printed another kind's label would be a board a player reads wrong
/// and the engine reads right — the worst shape a view bug takes. The
/// P/T pair is open-ended (CR 122.1a), so its badge is built from its
/// numbers, and the doc's claim is that the two ordinary ones are still
/// handed back without allocating: `+1/+1` and `-1/-1` are `Borrowed`
/// and `+2/+2` is not.
#[test]
fn every_counter_prints_a_badge_of_its_own_and_two_of_them_are_free() {
    let every = [
        CounterKind::PLUS_ONE,
        CounterKind::MINUS_ONE,
        CounterKind::Loyalty,
        CounterKind::Lore,
        CounterKind::Time,
        CounterKind::Charge,
        CounterKind::Poison,
        CounterKind::Energy,
        CounterKind::Rad,
        CounterKind::Lifelink,
        CounterKind::Level,
        CounterKind::Custom(9),
    ];
    let printed: Vec<String> = every.iter().map(|k| k.badge().into_owned()).collect();
    assert_eq!(
        printed,
        vec![
            "+1/+1", "-1/-1", "LOY", "LORE", "TIME", "CHG", "PSN", "NRG", "RAD", "LL", "LVL", "•",
        ]
    );
    assert_eq!(
        printed
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        printed.len(),
        "two kinds print one badge, so a player cannot tell them apart"
    );
    assert_eq!(
        every.iter().map(|k| counter_index(*k)).collect::<Vec<_>>(),
        (0..12).collect::<Vec<_>>(),
        "the list is every kind exactly once, in declaration order"
    );

    assert!(matches!(CounterKind::PLUS_ONE.badge(), Cow::Borrowed(_)));
    assert!(matches!(CounterKind::MINUS_ONE.badge(), Cow::Borrowed(_)));
    let odd = CounterKind::Plus {
        power: 2,
        toughness: 0,
    };
    assert_eq!(odd.badge(), "+2/+0");
    assert!(
        matches!(odd.badge(), Cow::Owned(_)),
        "a pair that is not one of the two common ones is built"
    );
    assert_eq!(
        CounterKind::Minus {
            power: 0,
            toughness: 1,
        }
        .badge(),
        "-0/-1",
        "and the asymmetric one the layer system needs prints both halves"
    );
}

/// A P/T counter is drawn on the card face and every other kind as a
/// badge beside it, so this is the question a client asks before it
/// draws anything — and the two constants are P/T counters like any
/// other pair.
#[test]
fn only_a_counter_that_changes_a_body_is_drawn_on_the_body() {
    for kind in [
        CounterKind::PLUS_ONE,
        CounterKind::MINUS_ONE,
        CounterKind::Plus {
            power: 3,
            toughness: 0,
        },
        CounterKind::Minus {
            power: 0,
            toughness: 2,
        },
    ] {
        assert!(kind.is_power_toughness(), "{kind:?}");
    }
    for kind in [
        CounterKind::Loyalty,
        CounterKind::Lore,
        CounterKind::Time,
        CounterKind::Charge,
        CounterKind::Poison,
        CounterKind::Energy,
        CounterKind::Rad,
        CounterKind::Lifelink,
        CounterKind::Level,
        CounterKind::Custom(1),
    ] {
        assert!(!kind.is_power_toughness(), "{kind:?}");
    }
}

/// The turn strip a client draws is twelve labels and they have to be
/// twelve: two steps sharing one would put the marker in a place the
/// player cannot read, and the strip is the only thing that says where
/// in the turn a game is.
#[test]
fn the_turn_strip_names_each_step_once() {
    const STRIP: [(Step, &str); 12] = [
        (Step::Untap, "UT"),
        (Step::Upkeep, "UP"),
        (Step::Draw, "DR"),
        (Step::Main, "M"),
        (Step::CombatBegin, "BC"),
        (Step::DeclareAttackers, "DA"),
        (Step::DeclareBlockers, "DB"),
        (Step::CombatDamageFirst, "FS"),
        (Step::CombatDamage, "CD"),
        (Step::CombatEnd, "EC"),
        (Step::End, "END"),
        (Step::Cleanup, "CL"),
    ];
    for (step, label) in STRIP {
        assert_eq!(step.short_label(), label, "{step:?}");
    }
    let labels: std::collections::BTreeSet<&str> = STRIP.iter().map(|(_, l)| *l).collect();
    assert_eq!(labels.len(), 12, "a label is used twice");

    // The six that light the combat lane, which is the other question a
    // client asks of a step and the reason the strip is not just text.
    let combat: Vec<&str> = STRIP
        .iter()
        .filter(|(step, _)| step.is_combat())
        .map(|(_, l)| *l)
        .collect();
    assert_eq!(combat, vec!["BC", "DA", "DB", "FS", "CD", "EC"]);
}

/// The stack is stored bottom first, so the object that resolves next is
/// the **last** entry. A client that drew it the other way round would
/// show a player the wrong answer to the one question a stack is for.
#[test]
fn the_top_of_the_stack_is_the_last_entry() {
    let mut view = view(2);
    assert!(view.top_of_stack().is_none(), "an empty stack has no top");

    view.stack.push(obj(10, 0));
    view.stack.push(obj(11, 1));
    assert_eq!(
        view.top_of_stack().map(|o| o.id),
        Some(ObjectId::new(11, 0)),
        "the response resolves before the spell it answered"
    );
    assert_eq!(
        view.object(ObjectId::new(10, 0)).map(|o| o.id),
        Some(ObjectId::new(10, 0)),
        "and the one underneath is still findable"
    );
}
