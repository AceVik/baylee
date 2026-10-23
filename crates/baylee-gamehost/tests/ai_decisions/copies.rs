#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// #214. A copy uses the abilities of the card it copied (CR 707.2).
///
/// The engine offers a copy's ability as an index into the copied card's
/// list, and the agent looked that index up on the card underneath: a
/// Glasspool Mimic that entered as a Werefox Bodyguard prints one ability of
/// its own, so the Fox's second came back as nothing and the copy was never
/// used. The view names the copied card in `rules` since `VIEW_VERSION` 28,
/// and this board is the one `combo_tests/printed_faces.rs` plays.
///
/// Three things are arranged by hand so the question is the only one on the
/// table. The original Fox is sacrificed first, so that the copy's is the
/// one offer. The `{1}{W}` is floated before the agent is asked, because the
/// engine offers an activation only against mana already in the pool —
/// without it the Mimic's ability is not on offer at all, and a pass would
/// say nothing about the agent. And the sacrifice is made worth making: at
/// five life, with the opponent's Path to Exile aimed at the copy, the copy
/// is gone anyway, so two life for `{1}{W}` is a gain `worth` takes. On an
/// empty stack it is not, a 2/2 being worth more than two life, which is
/// what the whitelist this test was written against used to get wrong.
///
/// If `worth` stops taking the sacrifice here, this goes red for a reason
/// that is not #214; the two `a_copy_*` tests in `baylee-ai` are the ones
/// that carry the lookup.
#[test]
fn a_copy_uses_the_ability_of_the_card_it_copied() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = position(
        &["Glasspool Mimic"],
        &[
            "Werefox Bodyguard",
            "Island",
            "Island",
            "Island",
            "Plains",
            "Plains",
            "Plains",
            "Plains",
        ],
    );
    preset.seats[0].starting_life = Some(5);
    preset.seats[1].starting_hand = Some(vec![entry("Path to Exile")]);
    preset.seats[1].starting_battlefield = vec![entry("Plains")];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();

    settle(&mut engine, &[]);
    let view = asked_view(engine.state(), me, 1, engine.pending());
    let fox = untapped(&view, "Werefox Bodyguard")[0];
    let mimic = view
        .hand
        .iter()
        .find(|c| c.card.index == entry("Glasspool Mimic").card)
        .expect("the Mimic in hand")
        .id;
    float(&mut engine, &untapped(&view, "Island"));
    engine
        .apply(me, PlayerAction::CastSpell { card: mimic })
        .expect("three Islands pay for the Mimic");
    // The Fox for the Mimic's copy, and nothing for the copied enters
    // trigger, which may exile only a creature that is not a Fox.
    settle(&mut engine, &[fox]);

    let view = asked_view(engine.state(), me, 2, engine.pending());
    let copy = untapped(&view, "Glasspool Mimic")[0];
    assert_eq!(
        view.object(copy).and_then(|o| o.rules).map(|r| r.card),
        Some(entry("Werefox Bodyguard").card),
        "the Mimic did not arrive as a copy of the Fox, so there is no copy \
         for the agent to read"
    );
    float(&mut engine, &untapped(&view, "Plains")[..2]);
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: fox,
                ability_index: 1,
            },
        )
        .expect("the original Fox sacrifices itself");
    settle(&mut engine, &[]);

    // The opponent's Path to Exile, aimed at the copy, and back to this seat.
    engine.apply(me, PlayerAction::PassPriority).unwrap();
    let view = asked_view(engine.state(), them, 3, engine.pending());
    let plains = view
        .battlefield_of(them)
        .find(|o| o.name == "Plains")
        .expect("their Plains")
        .id;
    let path = view.hand[0].id;
    engine
        .apply(them, PlayerAction::ActivateManaAbility { source: plains })
        .unwrap();
    engine
        .apply(them, PlayerAction::CastSpell { card: path })
        .expect("their Plains pays for Path to Exile");
    engine
        .apply(
            them,
            PlayerAction::ChooseObjects {
                objects: vec![copy],
            },
        )
        .expect("pointed at the copy");
    engine.apply(them, PlayerAction::PassPriority).unwrap();

    let view = asked_view(engine.state(), me, 4, engine.pending());
    assert_eq!(view.stack.len(), 1, "the Path is on the stack");
    float(&mut engine, &untapped(&view, "Plains")[..2]);
    let view = asked_view(engine.state(), me, 5, engine.pending());
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(
        legal.abilities,
        vec![(copy, 1)],
        "the engine must offer the copy the Fox's sacrifice and nothing else, \
         or what follows is not this question"
    );
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(&view, engine.pending()),
        PlayerAction::ActivateAbility {
            source: copy,
            ability_index: 1,
        },
        "the agent looked the Fox's ability up on the Mimic underneath"
    );
}

/// What seat 0's `clone` entered as, when the agent plays turn 1 with it in
/// hand beside `board`, against `theirs`.
fn cloned(profile: AIProfile, clone: &str, board: &[&str], theirs: &[&str]) -> Option<String> {
    let mut preset = position(&[clone], board);
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(profile);
    for seq in 0..200 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("nobody has lost on turn 1");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.turn > 1 {
            break;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("every planned action is legal");
    }
    let view = asked_view(engine.state(), PlayerId::new(0), 999, engine.pending());
    let copy = view
        .battlefield_of(PlayerId::new(0))
        .find(|o| o.card.is_some_and(|c| c.index == entry(clone).card))?;
    Some(baylee_cards::by_index(copy.rules?.card)?.name().to_owned())
}

/// #227. A clone copies what is worth having twice, whoever controls it.
///
/// The engine asked the clone's choice with no source and no effects, so the
/// agent answered it as it answers a removal spell — an opponent's permanent
/// first — and a Phyrexian Metamorph copied the opponent's Llanowar Elves
/// over its own controller's Serra Angel. Both directions are played: a
/// rule that preferred this seat's own permanents would pass the first
/// board and fail the second.
///
/// The legend rule (CR 704.5j) is the exception. A copy of a legendary
/// creature this seat already controls keeps one of the two, so the
/// Metamorph takes the Elves over Emry. Spark Double's copy is not
/// legendary, so it takes Emry over the Elves.
#[test]
fn a_clone_copies_what_is_worth_having_twice() {
    let islands = ["Island", "Island", "Island", "Island"];
    let with =
        |mine: &'static str| -> Vec<&'static str> { [mine].into_iter().chain(islands).collect() };
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        let cases: [(&str, Vec<&str>, &[&str], &str); 4] = [
            (
                "Phyrexian Metamorph",
                with("Serra Angel"),
                &["Llanowar Elves"],
                "Serra Angel",
            ),
            (
                "Phyrexian Metamorph",
                with("Llanowar Elves"),
                &["Serra Angel"],
                "Serra Angel",
            ),
            (
                "Phyrexian Metamorph",
                with("Emry, Lurker of the Loch"),
                &["Llanowar Elves"],
                "Llanowar Elves",
            ),
            (
                "Spark Double",
                [&with("Emry, Lurker of the Loch")[..], &["Llanowar Elves"]].concat(),
                &[],
                "Emry, Lurker of the Loch",
            ),
        ];
        for (clone, mine, theirs, wanted) in cases {
            assert_eq!(
                cloned(profile, clone, &mine, theirs).as_deref(),
                Some(wanted),
                "{profile:?}: {clone} beside {mine:?}, against {theirs:?}"
            );
        }
    }
}
