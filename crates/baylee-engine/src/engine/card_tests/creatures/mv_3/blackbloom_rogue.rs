//! `cards/creatures/mv_3/blackbloom_rogue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "This land enters tapped. {T}: Add {B}." — the back face, reached the way
/// CR 712.12 says a player reaches it: the card is **played as a land**, not
/// cast. The front face is a creature, so exactly one face is a land and the
/// engine switches to it with no question asked; the `ChooseCastMode` list
/// pathways get is for a card whose *both* faces are lands.
///
/// The tapped clause is why this has to be a real `PlayLand`. A Bog seeded
/// through `Duel::battlefield` is placed rather than entered, so no
/// replacement effect ever looks at it and the first untap step would have
/// untapped it anyway — a test resting on that would measure nothing. And the
/// modifier is printed on face **1**, so a reader that took its enter
/// modifiers off `faces[0]` would let the Bog make mana the turn it landed.
///
/// `{T}: Add {B}` is asked for through `legal.abilities` and **not**
/// `legal.mana_abilities`, and the difference is the card rather than the
/// kit. `legal.mana_abilities` is fed by `casting::can_activate_mana`, which
/// asks `intrinsic_mana` — the CR 305.6 shortcut, and it answers only for the
/// five basic land types. Blackbloom Bog prints plain `Land` and carries its
/// own `mana_ability!`, so it lives in `legal.abilities` at index 0 and is
/// pressed with `ActivateAbility`; `ActivateManaAbility { source }` is
/// refused outright for a source that list never named. A test written the
/// other way round asserts nothing on the tapped turn — the Bog is in
/// `mana_abilities` on no turn at all — and is refused on the untapped one.
#[test]
fn a_blackbloom_bog_enters_tapped_and_taps_for_black_once_it_has_untapped() {
    let p0 = PlayerId::new(0);
    let (mut engine, bog) = play_land_face(blackbloom_rogue(), 1)
        .expect("the back face is a land, and a land drop is the way to it");

    let obj = engine.state().object(bog).expect("the Bog is on the table");
    assert_eq!(obj.face_index, 1, "the land is the back face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Blackbloom Bog",
        "and the permanent is named after that face, not after the card"
    );
    assert!(
        obj.characteristics().types.contains(TypeSet::LAND),
        "it came down as a land and not as the {{2}}{{B}} Rogue on the front"
    );
    assert!(
        obj.status.contains(Status::TAPPED),
        "`This land enters tapped` — printed on face 1, which is the face the \
         enter modifiers have to be read from"
    );

    // Tapped, it makes nothing: the whole ability costs {T}, and `can_afford`
    // refuses a `CostPart::TapSelf` on a source that is already tapped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = legal.abilities;
    assert!(
        !offered.contains(&(bog, 0)),
        "a land that entered tapped is offered no {{T}} ability the turn it \
         landed: {offered:?}"
    );

    // Its controller's next turn: it untaps, and then it makes black mana.
    walk_the_game_until(&mut engine, |e| {
        e.state().turn.number > 1
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(!is_tapped(&engine, bog), "the untap step untapped it");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    let offered = legal.abilities;
    assert!(
        offered.contains(&(bog, 0)),
        "`{{T}}: Add {{B}}` is the back face's only printed ability and there \
         is a {{T}} to pay now: {offered:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bog,
                ability_index: 0,
            },
        )
        .expect("the offer is honoured");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one black mana, which is the whole of the back face's printed text — \
         and it is in the pool already, because a mana ability resolves \
         without the stack (CR 605.3b)"
    );
    assert!(is_tapped(&engine, bog), "and it paid {{T}} to make it");
}

/// "Menace. This creature gets +3/+0 as long as an opponent has eight or more
/// cards in their graveyard." — the front face cast for its printed {2}{B},
/// with the condition of the second sentence deliberately met.
///
/// Eight cards go into the opponent's graveyard *before* the Rogue is cast,
/// which is the threshold the card names exactly, and the creature that
/// arrives is still the 2/3 the type line prints. That is the other half of
/// `Coverage::Partial("the graveyard-threshold +3/+0 is never applied")`, and
/// the card leaves the modifier off rather than writing it unconditionally on
/// purpose: a bare `Modifier::ModifyPT(3, 0)` would be a permanent 5/3, which
/// is stronger than the printing, where omitting it only ever holds the Rogue
/// at the 2/3 it prints. A `StaticAbility` is a layer, a filter and a
/// modifier, and a `Filter` asks about an object, so there is nowhere to put
/// the *while* half of the sentence.
///
/// The day `pt` here reads `(5, 3)`, the condition has found a home and this
/// test is what says the `Coverage` flag on the card is now a lie.
#[test]
fn a_blackbloom_rogue_stays_a_two_three_though_their_graveyard_holds_eight() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[blackbloom_rogue()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    walk_the_game_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    seed_graveyard(&mut engine, p1, 8);
    // `eight **or more**`, and the assertion is written that way on purpose:
    // a seat that crossed a cleanup with a full hand on the way here discarded
    // into the same graveyard, which is nothing to do with the Rogue.
    assert!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len() >= 8,
        "the opponent has eight or more cards in their graveyard — the \
         printed threshold, met"
    );

    // One cast option and no question: `casting::castable_back_faces` skips a
    // land face, so the only way to cast this card is its front one.
    cast_from_hand(&mut engine, p0, blackbloom_rogue());
    pass_until(&mut engine, stack_is_empty);
    let rogue = on_battlefield(&engine, p0, blackbloom_rogue()).expect("the Rogue resolved");
    assert_eq!(
        engine
            .state()
            .object(rogue)
            .expect("it is on the table")
            .face_index,
        0,
        "cast out of the hand it is the creature face, not the land"
    );
    assert!(
        keywords(&engine, rogue).contains(KeywordSet::MENACE),
        "menace is printed on the front face and the layers project it"
    );
    assert_eq!(
        pt(&engine, rogue),
        (2, 3),
        "`gets +3/+0 as long as an opponent has eight or more cards in their \
         graveyard` is the clause the card admits it cannot say: the condition \
         holds and no modifier is applied. When this reads (5, 3) the static \
         has learned a condition and `Coverage::Partial` is what to fix"
    );
    let def = baylee_cards::by_index(blackbloom_rogue()).expect("the Rogue is in the pool");
    assert!(
        !def.is_implemented(),
        "and the card still says so, so the 2/3 above is a promise the \
         deckbuilder makes rather than a silent hole"
    );
}

/// "Menace (This creature can't be blocked except by two or more creatures.)"
/// — CR 702.111b, played out in combat, which is the only place the word
/// means anything.
///
/// Two untapped Elves stand across the table, so the sentence's own escape
/// clause is available: two or more creatures *may* block. Both halves of it
/// are asked here — the Rogue is on the offer that each Elf gets, and a lone
/// Elf is still refused.
///
/// This test used to say the opposite, and said it at length: the offer
/// named the Rogue to nobody and a *pair* was refused too, because
/// `combat::can_block` asked `state.combat.blockers_of(attacker)` while that
/// list was necessarily empty — both callers ask before anything is
/// recorded. Menace read as plain unblockable (#156). The assertion that
/// moved is the offer one; the one-blocker refusal is the printed line
/// holding and was green through both readings, which is why it is the one
/// that says nothing new.
///
/// That a pair is now *accepted* is a rules claim rather than a claim about
/// this card, so it lives beside the rule in `combat_choice_tests`. What
/// stays here is the card: the unblocked Rogue deals the 2 its type line
/// prints, and this card stays `Coverage::Partial` because what that flag
/// names is the +3/+0 and not menace.
#[test]
#[allow(clippy::too_many_lines)] // one attack, played step by step
fn a_blackbloom_rogue_s_menace_is_offered_to_both_blockers_and_refuses_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[blackbloom_rogue(), swamp()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    walk_the_game_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let rogue = on_battlefield(&engine, p0, blackbloom_rogue()).expect("the Rogue stands");
    assert_eq!(
        engine
            .state()
            .object(rogue)
            .expect("it is on the table")
            .face_index,
        0,
        "the preset seats the card on its front face, so the thing attacking \
         below is the creature and not the Bog"
    );
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two creatures are there to block with");

    // Their graveyard is full, which changes nothing about what the Rogue
    // hits for — the same gap the P/T test states, read as damage.
    seed_graveyard(&mut engine, p1, 8);
    let before = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&rogue),
        "a permanent the preset put out before turn one is not summoning sick"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p1, "the attack is aimed at them, so they block");
    assert_eq!(
        blockers.len(),
        2,
        "both untapped Elves are on the offer: {blockers:?}"
    );
    assert!(
        blockers.iter().all(|o| o.attackers.contains(&rogue)),
        "each Elf is paired with the Rogue, because either of them may be one \
         of the two CR 702.111b asks for — the restriction is on the whole \
         declaration (CR 509.1b) and the offer is per pair: {blockers:?}"
    );

    // One blocker is refused, and that is the printed line holding.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(elves[0], rogue)],
                },
            )
            .is_err(),
        "`can't be blocked except by two or more creatures`: a lone Elf is not \
         a legal block, and this assertion stays green when the one above it \
         goes"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declining to block is always legal");
    pass_until(&mut engine, |e| e.state().players[1].life < before);
    assert_eq!(
        engine.state().players[1].life,
        before - 2,
        "an unblocked Rogue deals the 2 its type line prints — not the 5 the \
         +3/+0 would have made of it, with eight cards lying in their yard"
    );
}
