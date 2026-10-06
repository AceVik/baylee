use super::*;

/// Which way an amount counts, asked of every variant there is.
///
/// This is the one place the question is answered, because it was four:
/// three closures in `resolve::counters` and one arm in the AI's tactics
/// each spelled `matches!(a, Amount::NegX | Amount::NegXFixed(_))` — a
/// positive list over an enum, four times over, and four chances for the
/// next negative amount to be read as a bonus. Nothing in a suite reads
/// a sign as a bug on its own: the card resolves, the creature changes
/// size, and only the direction is wrong.
///
/// All fifteen variants are named. That is a population rather than a
/// guard — the exhaustive `match` is what a new variant has to answer —
/// but the answers themselves are what no compiler can check.
#[test]
fn every_amount_says_which_way_it_counts() {
    const NEG: &Amount = &Amount::NegX;
    const POS: &Amount = &Amount::X;
    let downwards: Vec<Amount> = vec![Amount::NegX, Amount::NegXFixed(3), Amount::Negated(POS)];
    let upwards: Vec<Amount> = vec![
        Amount::Fixed(2),
        Amount::X,
        Amount::XPlusCommanderCasts,
        Amount::DoubleX,
        Amount::DistinctColorsAmong(&Filter::CREATURE),
        Amount::TargetPower,
        Amount::SourcePower,
        Amount::CountersOnSource(CounterKind::Charge),
        Amount::TargetCmc,
        Amount::SacrificedManaValue,
        Amount::ManaSpentToCast,
        Amount::CountOf {
            filter: &Filter::CREATURE,
            zone: ZoneSel::Battlefield,
        },
        // A negated negative: parity rather than a refusal, because that
        // is what the word means. No card prints a double negative, and
        // the rule is cheaper than a refusal that has to be remembered.
        Amount::Negated(NEG),
    ];
    assert_eq!(
        downwards.len() + upwards.len(),
        16,
        "sixteen values over fifteen variants — `Negated` is in both \
         lists, which is the parity rule being read from both sides"
    );
    for a in downwards {
        assert!(a.is_negative(), "{a:?} counts downwards");
    }
    for a in upwards {
        assert!(!a.is_negative(), "{a:?} does not count downwards");
    }
}

/// "Up to one target" is not one target, and reading it as one is an
/// ability a player cannot activate at all on an empty board — for
/// Teferi, Time Raveler a card that cannot be drawn, and for Karn, the
/// Great Creator a loyalty tick that cannot be taken. The four
/// constructors are the whole vocabulary a card has for saying how many.
#[test]
fn how_many_targets_a_card_asks_for() {
    const WHAT: TargetSpec = TargetSpec::Object(&Filter::CREATURE);

    let one = TargetReq::one(WHAT);
    assert_eq!((one.min, one.max), (1, 1));
    assert!(!one.count_is_x);

    let maybe = TargetReq::up_to_one(WHAT);
    assert_eq!(
        (maybe.min, maybe.max),
        (0, 1),
        "a minimum of nought is the whole of what 'up to' means"
    );
    assert_ne!(
        maybe, one,
        "the two are different requirements and not a spelling"
    );

    let two = TargetReq::up_to(WHAT, 2);
    assert_eq!((two.min, two.max), (0, 2));
    assert_eq!(
        TargetReq::up_to(WHAT, 1),
        maybe,
        "'up to one' is 'up to' with a one in it"
    );

    let x = TargetReq::x_targets(WHAT);
    assert_eq!(
        (x.min, x.max, x.count_is_x),
        (0, 255, true),
        "an X count is unbounded until X is announced"
    );
    let pair = TargetReq::exactly(WHAT, 2);
    assert_eq!((pair.min, pair.max), (2, 2), "'two target lands' is two");
    assert_eq!(
        TargetReq::exactly(WHAT, 1),
        one,
        "'one' is 'exactly' with a one in it"
    );

    for req in [one, maybe, two, x, pair] {
        assert_eq!(req.spec, WHAT, "each of them targets what it was given");
    }
}

/// `bounds` is the one reading of "how many" a cast and an activation
/// share: a fixed count is its own bounds, and an X count is X once X has
/// been announced.
#[test]
fn bounds_read_x_only_where_the_count_is_x() {
    const WHAT: TargetSpec = TargetSpec::Object(&Filter::CREATURE);
    assert_eq!(
        TargetReq::up_to(WHAT, 2).bounds(7),
        (0, 2),
        "X is not this count"
    );
    assert_eq!(TargetReq::exactly(WHAT, 2).bounds(0), (2, 2));
    assert_eq!(TargetReq::exactly(WHAT, 255).bounds(0), (255, 255));
    assert_eq!(TargetReq::up_to(WHAT, 255).bounds(0), (0, u32::MAX));
    let x = TargetReq::x_targets(WHAT);
    assert_eq!(x.bounds(0), (0, 0), "X = 0 targets nothing");
    assert_eq!(x.bounds(3), (3, 3), "X targets is exactly X");
    assert_eq!(
        x.bounds(300),
        (300, 300),
        "X target counts retain the full announced value"
    );
}
