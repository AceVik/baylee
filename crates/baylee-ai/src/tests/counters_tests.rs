use super::*;

/// Urza's Saga at chapter I, one on each side of the table.
///
/// A lore counter carries no sign of its own: it advances whichever Saga
/// it lands on, so the seat that wants it is that Saga's controller and
/// the opponent's copy is the one place it must not go. Before this,
/// `Time`, `Lore` and `Custom` all scored zero, `targets` declined to
/// express any preference, and the fallback — an opponent's permanents
/// first, which is right for almost every other spell — handed the
/// opponent their next chapter.
#[test]
fn a_lore_counter_goes_on_my_own_saga_and_never_the_opponents() {
    use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
    use baylee_engine::engine::DecisionContext;
    let v = view(
        0,
        &[20, 20],
        vec![
            saga(obj(1), PlayerId::new(1), 1),
            saga(obj(2), PlayerId::new(0), 1),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::AddCounter {
        kind: Counter::Lore,
        amount: Amount::Fixed(1),
    }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };

    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "a lore counter advances the Saga it lands on, so it belongs on my own"
    );
}

/// Two of the opponent's suspended cards, one of them a single upkeep
/// from casting itself for nothing.
///
/// A time counter delays whatever it sits on, so every suspended card is
/// a hostile target — but they are not equally hostile, and the fallback
/// could not tell them apart because it never ranked them: it took the
/// first enemy object it was offered. The clock about to run out is the
/// one worth another turn.
#[test]
fn a_time_counter_delays_the_suspended_card_that_is_about_to_cast() {
    use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
    use baylee_engine::engine::DecisionContext;
    let mut v = view(0, &[20, 20], vec![]);
    v.exile[1] = vec![
        suspended(obj(1), PlayerId::new(1), 4),
        suspended(obj(2), PlayerId::new(1), 1),
    ];
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::AddCounter {
        kind: Counter::Time,
        amount: Amount::Fixed(1),
    }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };

    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "one time counter left is one upkeep from a free cast; four is not"
    );
}

/// A +1/+1 counter is a gift everywhere except on a creature with
/// undying, where it is the thing that stops it coming back (CR 702.93a).
///
/// My own board, two creatures, and the bigger one is the Geist: without
/// this rule the counter goes to it, because `material` ranks a 2/2 over
/// a 1/1 and a friendly seat over every other consideration. The 1/1 is
/// what the agent has to take instead, and it has to take it for the
/// reason the card prints rather than by accident — which is why the
/// board is built the wrong way round on purpose.
#[test]
fn a_plus_one_counter_does_not_go_on_my_own_undying_creature() {
    use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect, KeywordSet};
    use baylee_engine::engine::DecisionContext;
    let me = PlayerId::new(0);
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), me, 1),
            keyworded(obj(2), me, 2, "Strangleroot Geist", KeywordSet::UNDYING),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::AddCounter {
        kind: Counter::P1P1,
        amount: Amount::Fixed(1),
    }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };

    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(1)],
            players: vec![]
        },
        "the smaller creature, because the bigger one would stop coming back"
    );
}

/// Bridgeworks Battle's second question: "It fights up to one target
/// creature you don't control", asked after my 1/1 was named for the
/// +2/+2.
///
/// The whole spell reads as beneficial, and read that way every creature
/// across the table is one the pump must not go to — so the agent
/// answered "up to one" with none and never fought. Asked per instance,
/// the 3/3 it will be kills their 2/2 and survives it; their 4/4 it
/// would not kill.
#[test]
fn a_fight_names_the_creature_its_fighter_kills_and_survives() {
    use baylee_cards_dsl::{Amount, Duration, Effect, KeywordSet, TargetSlot};
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), me, 1),
            permanent(obj(2), them, 2),
            permanent(obj(3), them, 4),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(2), obj(3)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [
        Effect::PumpTarget {
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
        Effect::Fight {
            fighter: TargetSlot::First,
            foe: TargetSlot::Second,
        },
    ];
    let context = DecisionContext {
        effects: &effects,
        second_instance: true,
        first_targets: &[obj(1)],
        ..Default::default()
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "the 2/2 dies to the pumped 3/3 and deals it only two"
    );

    // The same question with nothing it can kill: "up to one" is
    // declined rather than spent on a bout that only loses the fighter.
    let v = view(
        0,
        &[20, 20],
        vec![permanent(obj(1), me, 1), permanent(obj(3), them, 6)],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(3)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![]
        },
        "a 3/3 fighting a 6/6 kills nothing and dies"
    );
}

/// Khalni Ambush's first question: which of my creatures fights. The
/// spell has no benefit and no damage of its own to rank by, so the
/// general ranking had no opinion and the fallback named whichever
/// creature was listed first — here the 1/1, which dies to their 2/2
/// without killing it. The 4/4 is the fighter with a fight worth having.
#[test]
fn a_fight_names_the_fighter_with_a_fight_worth_having() {
    use baylee_cards_dsl::{Effect, TargetSlot};
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), me, 1),
            permanent(obj(2), me, 4),
            permanent(obj(3), them, 2),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::Fight {
        fighter: TargetSlot::First,
        foe: TargetSlot::Second,
    }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "the 4/4 kills the 2/2 and lives"
    );
}

/// The same counter, the same rule, read off the other card — and this
/// is the half that says the agent is reading the *object* and not
/// applying a second fixed sign.
///
/// A −1/−1 counter is hostile wherever it goes, so both of the
/// opponent's creatures are legitimate targets and `material` picks the
/// 3/3. Persist (CR 702.79a) is what makes the 1/1 Elite worth more: the
/// counter kills it *and* is the counter its own return would have
/// given it, so it does not come back. The pair of tests is the point —
/// a rule that answered "+1/+1 is bad here" would pass the first one and
/// fail this.
#[test]
fn a_minus_one_counter_prefers_the_persist_creature_it_keeps_down() {
    use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect, KeywordSet};
    use baylee_engine::engine::DecisionContext;
    let them = PlayerId::new(1);
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), them, 3),
            keyworded(obj(2), them, 1, "Safehold Elite", KeywordSet::PERSIST),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::AddCounter {
        kind: Counter::M1M1,
        amount: Amount::Fixed(1),
    }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };

    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "the 1/1 that persist would otherwise return, not the bigger body"
    );
}

/// #166. A time counter is only a *delay* on a card that is counting
/// down, and the rule asks the card rather than the counter.
///
/// Trenzalore Clocktower is the pool's first card that puts a time
/// counter on anything — `{T}: Add {U}. Put a time counter on Trenzalore
/// Clocktower.` — and its counters run the other way from suspend: a
/// private count to twelve on a land, where more is better for the
/// controller. The worry that came with it was that `clock_score` would
/// score it backwards, since its `Time` arm is written for suspend.
///
/// It does not. The arm asks whether the card underneath prints
/// `Suspend` and answers 0 when it does not, which is the same refusal
/// it already gives vanishing. The counters are the deciding part of
/// this fixture: the Clocktower carries **three** and the suspended card
/// **four**, so a rule that read the count alone — one upkeep sooner is
/// worth more — would take the Clocktower. Taking Ancestral Vision is
/// what says the card is being read and not just its counters.
#[test]
fn a_time_counter_is_not_a_delay_on_a_card_that_is_not_counting_down() {
    use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
    use baylee_engine::engine::DecisionContext;
    let enemy = PlayerId::new(1);
    let mut clocktower = carded(
        permanent(obj(1), enemy, 0),
        "Trenzalore Clocktower",
        TypeSet::LAND,
    );
    clocktower.name = "Trenzalore Clocktower".into();
    clocktower.power = None;
    clocktower.toughness = None;
    clocktower.counters = vec![CounterEntry {
        kind: CounterKind::Time,
        count: 3,
    }];
    let mut v = view(0, &[20, 20], vec![clocktower]);
    v.exile[1] = vec![suspended(obj(2), enemy, 4)];
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::AddCounter {
        kind: Counter::Time,
        amount: Amount::Fixed(1),
    }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "the Clocktower was read as a clock running out, and it is a \
         clock the land's controller is winding up"
    );
}
