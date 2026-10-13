use super::*;
use crate::manaplan::Dear;
use baylee_core::ids::GrantedActionId;
use baylee_engine::choice::{GrantedActionKind, GrantedActionOffer};

fn id(n: u32) -> ObjectId {
    ObjectId::new(n, 0)
}

fn cost(src: &str) -> ManaCost {
    ManaCost::try_parse(src).expect("a valid cost")
}

fn land(n: u32, color: ManaColor) -> Source {
    Source::fixed(id(n), Tap::Intrinsic, color)
}

fn step(n: u32) -> Step {
    Step {
        source: id(n),
        tap: Tap::Intrinsic,
        color: None,
    }
}

fn staged(steps: &[Step]) -> Staging {
    let mut staging = Staging::default();
    for step in steps {
        staging.toggle(*step);
    }
    staging
}

fn sources_of(staging: &Staging) -> Vec<ObjectId> {
    staging.steps().iter().map(|s| s.source).collect()
}

/// Two Forests and a Plains, the board the cost tests share.
fn three_lands() -> Vec<Source> {
    vec![
        land(1, ManaColor::Green),
        land(2, ManaColor::Green),
        land(3, ManaColor::White),
    ]
}

/// The painland shape the planner keeps: a free `{C}` and a dear `{W}`/`{U}`.
fn painland(n: u32) -> Source {
    Source {
        dear: Some(Dear {
            tap: Tap::Ability(1),
            colors: vec![ManaColor::White, ManaColor::Blue],
        }),
        ..Source::fixed(id(n), Tap::Ability(0), ManaColor::Colorless)
    }
}

fn five_colour(n: u32) -> Source {
    Source {
        colors: ManaColor::ALL[..5].to_vec(),
        ..Source::fixed(id(n), Tap::Ability(0), ManaColor::White)
    }
}

fn pool_of(green: u32) -> ManaPoolView {
    ManaPoolView {
        green,
        ..ManaPoolView::default()
    }
}

/// Staging a permanent adds it behind the others, in the order clicked; the
/// order is the order the taps are sent in, so it must never be sorted.
#[test]
fn toggle_stages_in_the_order_clicked() {
    let mut staging = Staging::default();
    assert!(staging.is_empty());
    assert!(staging.toggle(step(7)), "a new permanent is staged");
    assert!(staging.toggle(step(3)));
    assert!(staging.toggle(step(5)));
    assert_eq!(sources_of(&staging), [id(7), id(3), id(5)]);
    assert!(!staging.is_empty());
    assert!(staging.holds(id(3)));
    assert!(!staging.holds(id(4)));
}

/// A second toggle of the same permanent takes the tap back even when it
/// asks for another mode or colour: a permanent taps once, so the other
/// colour is a take-back and a fresh stage, never a second step.
#[test]
fn toggling_a_staged_permanent_again_unstages_it_whatever_it_asks() {
    let mut staging = staged(&[step(1), step(2), step(3)]);
    let other_mode = Step {
        source: id(2),
        tap: Tap::Ability(1),
        color: Some(ManaColor::Blue),
    };
    assert!(!staging.toggle(other_mode), "the second click un-stages");
    assert_eq!(sources_of(&staging), [id(1), id(3)]);
    assert!(!staging.holds(id(2)));
    assert!(staging.toggle(step(2)), "and a third click stages it again");
    assert_eq!(
        sources_of(&staging),
        [id(1), id(3), id(2)],
        "re-staged at the back"
    );
}

/// `unstage` removes one permanent and says whether it was there; `clear`
/// removes everything.
#[test]
fn unstage_and_clear_take_taps_back() {
    let mut staging = staged(&[step(1), step(2)]);
    assert!(staging.unstage(id(1)));
    assert!(!staging.unstage(id(1)), "already gone");
    assert!(!staging.unstage(id(9)), "never staged");
    assert_eq!(sources_of(&staging), [id(2)]);
    staging.clear();
    assert!(staging.is_empty());
    assert!(staging.steps().is_empty());
    assert!(!staging.holds(id(2)));
}

/// A Sol Ring shaped source (one colour, `amount: 2`) is worth two of its
/// colour for one tap.
#[test]
fn worth_of_a_fixed_source_counts_its_amount() {
    let sol_ring = Source {
        amount: 2,
        ..Source::fixed(id(1), Tap::Ability(0), ManaColor::Colorless)
    };
    let tap = Step {
        source: id(1),
        tap: Tap::Ability(0),
        color: None,
    };
    assert_eq!(worth(&tap, &[sol_ring]), [(ManaColor::Colorless, 2)]);
}

/// A bundle source makes every colour it lists at once, one of each.
#[test]
fn worth_of_a_bundle_is_one_of_each_colour() {
    let karoo = Source {
        colors: vec![ManaColor::White, ManaColor::Blue],
        bundle: true,
        ..Source::fixed(id(1), Tap::Ability(0), ManaColor::White)
    };
    let tap = Step {
        source: id(1),
        tap: Tap::Ability(0),
        color: None,
    };
    assert_eq!(
        worth(&tap, &[karoo]),
        [(ManaColor::White, 1), (ManaColor::Blue, 1)]
    );
}

/// A choice source is worth the colour the step answers with, and nothing
/// when the step has not answered: the estimate never guesses a colour.
#[test]
fn worth_of_a_choice_needs_the_colour_answered() {
    let sources = [five_colour(1)];
    let red = Step {
        source: id(1),
        tap: Tap::Ability(0),
        color: Some(ManaColor::Red),
    };
    let unanswered = Step { color: None, ..red };
    assert_eq!(worth(&red, &sources), [(ManaColor::Red, 1)]);
    assert!(worth(&unanswered, &sources).is_empty());
}

/// A painland staged in its dear mode is worth the coloured mana, staged in
/// its free mode the colourless one.
#[test]
fn worth_of_a_painland_follows_the_mode_staged() {
    let sources = [painland(1)];
    let dear = Step {
        source: id(1),
        tap: Tap::Ability(1),
        color: Some(ManaColor::White),
    };
    let free = Step {
        source: id(1),
        tap: Tap::Ability(0),
        color: None,
    };
    assert_eq!(worth(&dear, &sources), [(ManaColor::White, 1)]);
    assert_eq!(worth(&free, &sources), [(ManaColor::Colorless, 1)]);
    let unanswered = Step {
        color: None,
        ..dear
    };
    assert!(
        worth(&unanswered, &sources).is_empty(),
        "the dear mode asks a colour, and none was given"
    );
}

/// What this client cannot read is worth nothing, so the remainder
/// over-states the debt: an unknown source, a colour the source cannot
/// make, and a mode the source does not list.
#[test]
fn worth_never_over_counts_what_it_cannot_read() {
    let sources = [land(1, ManaColor::Green), five_colour(2)];
    let not_listed = Step {
        source: id(99),
        tap: Tap::Intrinsic,
        color: None,
    };
    assert!(worth(&not_listed, &sources).is_empty(), "no such source");
    let wrong_colour = Step {
        source: id(1),
        tap: Tap::Intrinsic,
        color: Some(ManaColor::Red),
    };
    assert!(
        worth(&wrong_colour, &sources).is_empty(),
        "a Forest cannot be asked for red"
    );
    let colourless_from_five = Step {
        source: id(2),
        tap: Tap::Ability(0),
        color: Some(ManaColor::Colorless),
    };
    assert!(
        worth(&colourless_from_five, &sources).is_empty(),
        "a five-colour source does not list colourless"
    );
    let wrong_mode = Step {
        source: id(1),
        tap: Tap::Ability(5),
        color: None,
    };
    assert!(worth(&wrong_mode, &sources).is_empty(), "no such mode");
}

/// Nothing staged leaves the whole cost owed; each staged tap takes its
/// worth off it; once the staged taps pay it, `remaining` is empty and
/// `covers` is true.
#[test]
fn remaining_and_covers_follow_the_staged_taps() {
    let sources = three_lands();
    let owed = cost("{2}{G}");
    let none = Staging::default();
    assert_eq!(
        none.remaining(&owed, &ManaPoolView::default(), &sources),
        [ManaSymbol::Generic(2), ManaSymbol::Green]
    );
    assert!(!none.covers(&owed, &ManaPoolView::default(), &sources));

    let one = staged(&[step(1)]);
    assert_eq!(
        one.remaining(&owed, &ManaPoolView::default(), &sources),
        [ManaSymbol::Generic(2)],
        "a staged Forest pays the green"
    );
    assert!(!one.covers(&owed, &ManaPoolView::default(), &sources));

    let all = staged(&[step(1), step(2), step(3)]);
    assert!(
        all.remaining(&owed, &ManaPoolView::default(), &sources)
            .is_empty()
    );
    assert!(all.covers(&owed, &ManaPoolView::default(), &sources));
}

/// Mana already floating counts beside the staged taps.
#[test]
fn floating_mana_and_staged_taps_cover_together() {
    let sources = three_lands();
    let owed = cost("{2}{G}");
    let two_others = staged(&[step(2), step(3)]);
    assert!(two_others.covers(&owed, &pool_of(1), &sources));
    assert!(
        two_others
            .remaining(&owed, &pool_of(1), &sources)
            .is_empty()
    );
    assert!(
        !two_others.covers(&owed, &ManaPoolView::default(), &sources),
        "without the floating green, two lands are one short"
    );
}

/// `pool_with` adds each staged tap's worth to the pool and leaves the
/// original pool as it was.
#[test]
fn pool_with_adds_the_worth_of_every_staged_tap() {
    let sources = three_lands();
    let staging = staged(&[step(1), step(3)]);
    let out = staging.pool_with(&pool_of(2), &sources);
    assert_eq!((out.green, out.white), (3, 1));
}

/// "Pay now" plans only the rest and never names a staged permanent.
#[test]
fn suggest_plans_the_rest_over_the_unstaged_sources() {
    let sources = three_lands();
    let owed = cost("{2}{G}");
    let staging = staged(&[step(1)]);
    let plan = staging
        .suggest(&owed, &ManaPoolView::default(), &sources)
        .expect("the other two lands pay the generic");
    let mut taps: Vec<ObjectId> = plan.steps.iter().map(|s| s.source).collect();
    taps.sort();
    assert_eq!(taps, [id(2), id(3)]);
    assert!(
        !plan.steps.iter().any(|s| staging.holds(s.source)),
        "a staged source is never planned again"
    );
}

/// When the unstaged sources cannot pay the rest there is no suggestion at
/// all, and when the staged taps already pay everything the suggestion is an
/// empty plan, not `None`.
#[test]
fn suggest_is_none_when_short_and_empty_when_covered() {
    let sources = three_lands();
    let staging = staged(&[step(1)]);
    assert!(
        staging
            .suggest(&cost("{G}{G}{G}"), &ManaPoolView::default(), &sources)
            .is_none(),
        "one Forest staged, one Forest and a Plains left for {{G}}{{G}}"
    );
    let all = staged(&[step(1), step(2), step(3)]);
    let plan = all
        .suggest(&cost("{2}{G}"), &ManaPoolView::default(), &sources)
        .expect("nothing left to pay is still a plan");
    assert!(plan.is_empty());
    let pooled = Staging::default()
        .suggest(&cost("{G}"), &pool_of(1), &sources)
        .expect("floating mana pays");
    assert!(pooled.is_empty());
}

fn offered(mana: &[u32], abilities: &[(u32, u32)]) -> LegalActions {
    LegalActions {
        mana_abilities: mana.iter().map(|&n| id(n)).collect(),
        abilities: abilities.iter().map(|&(n, i)| (id(n), i)).collect(),
        ..LegalActions::default()
    }
}

/// An intrinsic tap becomes `ActivateManaAbility`, a printed one
/// `ActivateAbility` with its index, a chosen colour follows its tap
/// directly, and the order is the staging order.
#[test]
fn wire_turns_each_step_into_its_action_in_staging_order() {
    let ability = Step {
        source: id(2),
        tap: Tap::Ability(3),
        color: Some(ManaColor::Blue),
    };
    let staging = staged(&[ability, step(1)]);
    let wire = staging
        .wire(&offered(&[1], &[(2, 3)]))
        .expect("both taps are offered");
    assert_eq!(
        wire,
        [
            PlayerAction::ActivateAbility {
                source: id(2),
                ability_index: 3
            },
            PlayerAction::ChooseColor(ManaColor::Blue),
            PlayerAction::ActivateManaAbility { source: id(1) },
        ]
    );
}

/// A tap the engine no longer offers makes the whole wire `None` (the
/// payment is all-or-nothing), an index offered for another ability does not
/// count, and an empty staging is an empty payment.
#[test]
fn wire_refuses_a_tap_that_is_no_longer_offered() {
    let staging = staged(&[step(1), step(2)]);
    assert_eq!(staging.wire(&offered(&[1], &[])), None, "Forest 2 is gone");
    let ability = staged(&[Step {
        source: id(2),
        tap: Tap::Ability(1),
        color: None,
    }]);
    assert_eq!(
        ability.wire(&offered(&[2], &[(2, 0)])),
        None,
        "ability 0 offered, ability 1 staged; an intrinsic offer is no help"
    );
    assert_eq!(
        Staging::default().wire(&offered(&[], &[])),
        Some(Vec::new())
    );
}

/// `keep_offered` drops exactly the taps no longer offered, keeps the order
/// of the rest, and reports whether it dropped any.
#[test]
fn keep_offered_drops_exactly_the_taps_that_went() {
    let mut staging = staged(&[step(1), step(2), step(3)]);
    assert!(!staging.keep_offered(&offered(&[1, 2, 3], &[])), "all stay");
    assert_eq!(staging.steps().len(), 3);
    assert!(staging.keep_offered(&offered(&[1, 3], &[])));
    assert_eq!(sources_of(&staging), [id(1), id(3)]);
    assert!(staging.keep_offered(&offered(&[], &[])));
    assert!(staging.is_empty());
    assert!(!staging.keep_offered(&offered(&[], &[])), "nothing to drop");
}

/// `extend` appends a plan's taps behind what is staged and skips a
/// permanent that is already staged.
#[test]
fn extend_appends_a_plan_and_skips_what_is_staged() {
    let mut staging = staged(&[step(5)]);
    let plan = Plan {
        steps: vec![step(6), step(5), step(7)],
        cost: cost("{3}"),
    };
    staging.extend(&plan);
    assert_eq!(sources_of(&staging), [id(5), id(6), id(7)]);
}

/// A granted special action (`TakeGrantedAction`) that adds mana.
fn granted_offer(
    n: u64,
    cost: baylee_cards_dsl::SpecialActionCost,
    effect: GrantedActionKind,
) -> GrantedActionOffer {
    GrantedActionOffer {
        id: GrantedActionId::new(n),
        source: baylee_core::ids::DamageSourceRef {
            object: id(77),
            version: 0,
        },
        ability: None,
        timing: baylee_cards_dsl::SpecialActionTiming::ManaAbility,
        cost,
        effect,
    }
}

fn adds(color: ManaColor, amount: u16) -> GrantedActionKind {
    GrantedActionKind::AddMana { color, amount }
}

fn life_mana(n: u64, color: ManaColor, amount: u16) -> GrantedMana {
    GrantedMana::of(&granted_offer(
        n,
        baylee_cards_dsl::SpecialActionCost::Life(1),
        adds(color, amount),
    ))
    .expect("a life-paid fixed AddMana stages")
}

fn granted_legal(ids: &[u64]) -> LegalActions {
    LegalActions {
        granted_actions: ids
            .iter()
            .map(|&n| {
                granted_offer(
                    n,
                    baylee_cards_dsl::SpecialActionCost::Life(1),
                    adds(ManaColor::White, 1),
                )
            })
            .collect(),
        ..LegalActions::default()
    }
}

/// Only a fixed `AddMana` paid with life can be staged: a mana-costed offer
/// would spend the pool the payment is counting, and any other effect makes
/// no mana at all.
#[test]
fn granted_mana_of_reads_life_paid_add_mana_and_nothing_else() {
    use baylee_cards_dsl::SpecialActionCost;
    let staged = GrantedMana::of(&granted_offer(
        9,
        SpecialActionCost::Life(2),
        adds(ManaColor::Blue, 3),
    ));
    assert_eq!(
        staged,
        Some(GrantedMana {
            id: GrantedActionId::new(9),
            color: ManaColor::Blue,
            amount: 3,
        })
    );
    assert_eq!(
        GrantedMana::of(&granted_offer(
            9,
            SpecialActionCost::Mana(cost("{1}")),
            adds(ManaColor::Blue, 3),
        )),
        None,
        "a mana cost is not staged"
    );
    assert_eq!(
        GrantedMana::of(&granted_offer(
            9,
            SpecialActionCost::Life(1),
            GrantedActionKind::CancelDelayedLifeLoss { amount: 1 },
        )),
        None,
        "an effect that makes no mana is not staged"
    );
}

/// Toggling stages in order, again takes it back, and the answer says which.
#[test]
fn toggle_granted_stages_and_unstages() {
    let mut staging = Staging::default();
    let (a, b) = (
        life_mana(1, ManaColor::White, 1),
        life_mana(2, ManaColor::Blue, 1),
    );
    assert!(staging.toggle_granted(a));
    assert!(staging.toggle_granted(b));
    assert_eq!(staging.granted(), [a, b], "in the order staged");
    assert!(!staging.is_empty());
    assert!(!staging.toggle_granted(a), "again takes it back");
    assert_eq!(staging.granted(), [b]);
    assert!(staging.steps().is_empty(), "no tap was involved");
}

/// A staged granted action adds colour x amount to the pool, so it counts
/// toward `remaining` and `covers` beside the taps.
#[test]
fn a_staged_granted_action_counts_toward_the_payment() {
    let sources = three_lands();
    let owed = cost("{2}{W}");
    let mut staging = staged(&[step(1)]);
    staging.toggle_granted(life_mana(5, ManaColor::White, 2));

    let out = staging.pool_with(&ManaPoolView::default(), &sources);
    assert_eq!(
        (out.white, out.green),
        (2, 1),
        "colour x amount, and the tap"
    );
    assert!(staging.covers(&owed, &ManaPoolView::default(), &sources));
    assert!(
        staging
            .remaining(&owed, &ManaPoolView::default(), &sources)
            .is_empty()
    );

    let short = cost("{3}{W}");
    assert!(!staging.covers(&short, &ManaPoolView::default(), &sources));
    assert_eq!(
        staging.remaining(&short, &ManaPoolView::default(), &sources),
        [ManaSymbol::Generic(1)],
        "one generic is still owed"
    );

    staging.toggle_granted(life_mana(5, ManaColor::White, 2));
    assert!(
        !staging.covers(&owed, &ManaPoolView::default(), &sources),
        "taken back, it no longer pays"
    );
}

/// The wire: every tap step first, in staging order, then each granted
/// action as `TakeGrantedAction`; an offer that has gone makes it `None`.
#[test]
fn wire_appends_granted_actions_after_the_taps_and_refuses_a_gone_offer() {
    let mut staging = Staging::default();
    staging.toggle_granted(life_mana(7, ManaColor::White, 1));
    staging.toggle(step(1));
    staging.toggle_granted(life_mana(8, ManaColor::White, 1));
    let mut legal = granted_legal(&[7, 8]);
    legal.mana_abilities = vec![id(1)];
    assert_eq!(
        staging.wire(&legal),
        Some(vec![
            PlayerAction::ActivateManaAbility { source: id(1) },
            PlayerAction::TakeGrantedAction {
                id: GrantedActionId::new(7)
            },
            PlayerAction::TakeGrantedAction {
                id: GrantedActionId::new(8)
            },
        ]),
        "taps first even though a granted action was staged before them"
    );
    let mut gone = granted_legal(&[8]);
    gone.mana_abilities = vec![id(1)];
    assert_eq!(staging.wire(&gone), None, "offer 7 is gone: all or nothing");
}

/// `keep_offered` drops a granted entry whose offer left `legal`, counts it
/// as a drop, and keeps the rest in order.
#[test]
fn keep_offered_drops_a_granted_action_that_is_no_longer_offered() {
    let mut staging = Staging::default();
    let b = life_mana(8, ManaColor::White, 1);
    staging.toggle_granted(life_mana(7, ManaColor::White, 1));
    staging.toggle_granted(b);
    assert!(!staging.keep_offered(&granted_legal(&[7, 8])), "both stay");
    assert!(staging.keep_offered(&granted_legal(&[8])), "7 went");
    assert_eq!(staging.granted(), [b]);
    assert!(staging.keep_offered(&granted_legal(&[])));
    assert!(staging.is_empty());
}

/// `clear` takes granted actions back too, and a staging with only a granted
/// action is not empty.
#[test]
fn clear_and_is_empty_include_granted_actions() {
    let mut staging = Staging::default();
    staging.toggle_granted(life_mana(7, ManaColor::White, 1));
    assert!(!staging.is_empty());
    staging.clear();
    assert!(staging.is_empty());
    assert!(staging.granted().is_empty());
}
