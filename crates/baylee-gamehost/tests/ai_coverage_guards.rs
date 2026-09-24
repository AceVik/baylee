//! The coverage table in `docs/ai-coverage-todo.md`, as tripwires.
//!
//! That table lists the AI tests this pool cannot carry yet, one row per
//! mechanic. As prose it is a second truth beside the code: nobody reads it
//! and the compiler does not check it — which is the argument that turned
//! `data/card-index.tsv` into a compiled table, and the reason
//! `no_card_claims_a_keyword_the_engine_ignores` is a test rather than a
//! paragraph. A row that only a person can notice is a row nobody notices.
//!
//! So every row asks the **compiled pool** whether its precondition has
//! arrived, and the failure message says what is then owed. Two shapes, and
//! the difference between them is the whole point:
//!
//! - A mechanic the pool cannot yet print asserts **zero**. It is green
//!   today and fails on the day somebody implements that mechanic, naming
//!   the test that has become writable. This is the "TODO test" in the only
//!   form that works here: an `#[ignore]` never runs, and a green test that
//!   asserts nothing is worse than none at all.
//! - A mechanic the pool already prints asserts **more than zero**. It never
//!   fires on growth — seven planeswalkers becoming nine is not news — but it
//!   records in code that the row is owed *now* rather than later, and it
//!   fails if the mechanic ever leaves the pool and quietly takes the debt
//!   with it.
//!
//! The pool is read through `baylee_cards::all()` and never by grepping the
//! card files. Scouting this table with a regular expression first reported
//! **no** two-faced cards, where the compiled pool has 120: a textual probe
//! answers a question it cannot see, which is the same fault `knob` exists
//! to prevent in `xtask`. The same scouting pass then missed the one card in
//! the pool that *is* a Partner commander, because `partner` is a field on
//! `CardDef` and not a bit in `KeywordSet`: the first run of this test is
//! what found Sakashima of a Thousand Faces, and it found it by asking the
//! type rather than the spelling.
//!
//! Four things in that table have no probe here, and the reasons are
//! three different ones.
//!
//! **Proliferate, madness and meld are words the DSL cannot say at all.**
//! There is no `Effect` for proliferate and no handle anywhere for the other
//! two, so the thing that would have to change first is the vocabulary
//! rather than a card. A probe would assert zero against a population that
//! can never be anything else, which is a green test measuring nothing.
//!
//! **Infect and wither can be said and are not printed**, so they *are*
//! probed — in the tripwire test above, where a zero is the finding.
//!
//! **Full deck families and full-game lookahead are not card shapes.** One
//! is a paired-seed benchmark and the other a property of the search model;
//! no count over `baylee_cards::all()` could open or close either, and
//! pretending otherwise would put a number beside a row that the number
//! says nothing about.

use baylee_cards::dsl::{
    AbilityDef, CardDef, CostPart, CounterKind, Effect, KeywordSet, Modifier, PartnerKind,
};
use baylee_core::types::TypeSet;

/// Every ability a card carries, card-level and on either face.
///
/// Both, because the split is a layout detail: a two-faced card puts its
/// abilities on the faces and a single-faced one may use either list, so a
/// probe reading one of them alone measures the layout instead of the card.
fn abilities(def: &'static CardDef) -> impl Iterator<Item = &'static AbilityDef> {
    def.abilities
        .iter()
        .chain(def.faces.iter().flat_map(|face| face.abilities.iter()))
}

fn count(probe: impl Fn(&'static CardDef) -> bool) -> usize {
    baylee_cards::all().filter(|def| probe(def)).count()
}

/// Cards that *reach* an effect this probe accepts, at any depth.
///
/// The descent is [`Effect::walk`]'s, so this reads what a card does inside
/// a `Sequence`, in either half of a kicker clause and behind "unless you
/// pay". A probe matching the top-level list alone would answer a narrower
/// question than the row it is standing for asks, and would answer it
/// without saying so — which is the fault #109 closed and not one to
/// reintroduce one file away from it.
fn count_reaching(probe: impl Fn(&'static Effect) -> bool) -> usize {
    baylee_cards::all()
        .filter(|def| {
            let mut hit = false;
            let mut seen = 0;
            for effects in abilities(def).flat_map(ability_effects) {
                Effect::walk(effects, &mut seen, &mut |effect| hit |= probe(effect));
            }
            hit
        })
        .count()
}

/// Every effect list an ability resolves through.
///
/// Exhaustive with no wildcard arm, for the reason `baylee_cards`'s own
/// lints give about `branches`: a new [`AbilityDef`] variant dropping into
/// `_ => Vec::new()` would leave the probe below reading nothing about it
/// and still reporting a number.
fn ability_effects(ability: &'static AbilityDef) -> Vec<&'static [Effect]> {
    match ability {
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => vec![effects],
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => {
            modes.iter().map(|mode| mode.effects).collect()
        }
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. }
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. } => Vec::new(),
    }
}

/// Every counter an effect list puts on anything.
///
/// The descent is [`Effect::walk`]'s, which is the crate that owns `Effect`
/// answering which effects carry another one. This file used to answer it
/// here, and was short by the two `&'static Effect` singles — so a counter
/// behind "unless you pay" was invisible, and `seen` was 2696 where it is
/// now 2731.
fn counters_put(effects: &'static [Effect], seen: &mut usize, found: &mut Vec<CounterKind>) {
    Effect::walk(effects, seen, &mut |effect| {
        if let Effect::AddCounter { kind, .. } | Effect::AddCounterFilter { kind, .. } = effect {
            found.push(*kind);
        }
    });
}

/// Every effect the whole pool reaches, and how many were visited.
fn pool_effects() -> (Vec<&'static Effect>, usize) {
    let mut seen = 0;
    let mut all = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            Effect::walk(effects, &mut seen, &mut |effect| all.push(effect));
        }
    }
    (all, seen)
}

/// The pool prints none of these, so the AI test each one would need cannot
/// be written against a real card yet. When one of these fails, the mechanic
/// has arrived and the named row of `docs/ai-coverage-todo.md` is due.
#[test]
fn a_mechanic_the_pool_cannot_print_yet_has_no_ai_test_to_write() {
    let rows: [(&str, &str, usize); 2] = [
        (
            "Commander pair rules (every pairing but plain Partner)",
            "a game pairing the commander kinds that are not plain Partner, \
             where each one's tax and combat damage stay separate and a \
             partner's damage never counts toward the other's 21",
            // Written as an exclusion rather than a list of the four, so a
            // pairing added to `PartnerKind` later is caught by this test
            // instead of being silently left out of it.
            count(|def| !matches!(def.partner, PartnerKind::None | PartnerKind::Partner)),
        ),
        (
            "Proliferate and replacement effects (the poison half)",
            "an AI decision that treats poison counters as hostile and \
             +1/+1 counters as friendly, rather than assigning one sign to \
             every counter kind",
            count(|def| {
                def.all_keywords()
                    .contains(KeywordSet::INFECT.union(KeywordSet::WITHER))
            }),
        ),
    ];

    for (row, owed, found) in rows {
        assert_eq!(
            found, 0,
            "{found} card(s) in the pool now reach `{row}` in \
             docs/ai-coverage-todo.md. That row is no longer a TODO: write \
             {owed}, then delete this entry. A mechanic that arrives \
             without its AI test is a strategy nobody measured."
        );
    }
}

/// The pool already prints these, so the row is owed **now**. The assertion
/// is "more than none" and not an exact number on purpose: a row does not
/// stop being due because a second card joined it, and pinning the count
/// would train whoever adds a card to bump a number instead of writing a
/// test.
#[test]
#[allow(clippy::too_many_lines)] // one entry per table row, in one readable list
fn a_mechanic_the_pool_already_prints_is_owed_now_and_not_later() {
    let rows: [(&str, usize); 28] = [
        (
            "Commander pair rules (plain Partner)",
            count(|def| matches!(def.partner, PartnerKind::Partner)),
        ),
        (
            // Blocking to keep one paid, #75: `a_walker_that_would_die_is_\
            // chumped_for_and_one_that_would_not_is_not` in `baylee-ai`.
            "Planeswalker survival",
            count(|def| def.faces.iter().any(|face| face.loyalty.is_some())),
        ),
        (
            "Modal and multi-target effects",
            count(|def| {
                abilities(def).any(|a| {
                    matches!(
                        a,
                        AbilityDef::ModalSpell { .. } | AbilityDef::ModalTriggered { .. }
                    )
                })
            }),
        ),
        (
            // Paid, 23.09.2026: `a_fight_names_the_creature_its_fighter_\
            // kills_and_survives` and `a_fight_names_the_fighter_with_a_\
            // fight_worth_having` in `baylee-ai`. The first was the defect:
            // Bridgeworks Battle's pump made the whole spell read as a
            // benefit, so every creature across the table scored as one the
            // spell must not reach and "up to one" was answered with none.
            // Both instances of "target" are asked separately (CR 115.3), and
            // `fight::fight_targets` answers each by what the fight would do.
            "Modal and multi-target effects (a fight's two instances)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::Fight { .. } | Effect::DamageEqualToPower { .. }
                )
            }),
        ),
        // **This counts compiled faces, not printings**, and the difference
        // is filed as #115: the predicate is `faces.len() >= 2`, the printed
        // figure is 121, and what a client actually wants is the ~107 with a
        // separate back *image*. An adventure or a split prints both halves
        // on one physical face and still answers yes here. The number is a
        // correct measurement of the predicate and would be a wrong answer
        // to "how many cards have two faces", which is why the row says
        // which it is.
        (
            "Transformation and alternate zones (a second compiled face)",
            count(|def| def.faces.len() >= 2),
        ),
        (
            "Transformation and alternate zones (adventure and disturb)",
            count(|def| def.faces.iter().any(|face| face.adventure || face.disturb)),
        ),
        (
            "Variable costs (convoke and delve)",
            count(|def| def.faces.iter().any(|face| face.convoke || face.delve)),
        ),
        (
            // Paid, #74: `an_optional_additional_cost_is_paid_when_the_pool_\
            // covers_it`, `the_kicker_is_floated_before_the_cast` and the
            // real-engine `a_kicker_and_a_waterbend_are_paid_when_the_mana_is_\
            // there`. Kicker and "you may waterbend" are both this field.
            "Variable costs (an optional additional cost)",
            count(|def| {
                def.faces
                    .iter()
                    .any(|face| !face.additional_costs.is_empty())
            }),
        ),
        (
            // Paid, #73: `a_lore_counter_goes_on_my_own_saga_and_never_the_\
            // opponents` in `baylee-ai`. The probe stays because the test
            // needs a Saga in the pool to be about anything — if Urza's Saga
            // leaves, that test starts proving nothing and this row says so.
            "Contextual counters (the lore half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::SagaChapter { .. }))),
        ),
        (
            // Suspend is the half of that row the pool can already test:
            // a time counter on a suspended card is a clock the AI wants to
            // run down, and the same counter on vanishing is one it does not.
            //
            // Paid, #73: `a_time_counter_delays_the_suspended_card_that_is_\
            // about_to_cast`. Vanishing is still owed and cannot be written:
            // no card here prints it, and `clock_score` scores that case 0
            // rather than guessing at it.
            "Contextual counters (the suspend half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Suspend { .. }))),
        ),
        (
            // Paid, 23.09.2026: `a_plus_one_counter_does_not_go_on_my_own_\
            // undying_creature` and `a_minus_one_counter_prefers_the_persist_\
            // creature_it_keeps_down` in `baylee-ai`. This row moved here
            // from the "cannot print yet" list the day the pool printed the
            // two keywords, which is what the other test is for.
            //
            // Both are needed and neither alone is the rule: the sign of the
            // counter is still its own, and what `tactics::denies_a_return`
            // adds is that the counter a keyword's intervening `if` reads is
            // a denial on the creature that prints it (CR 702.93a,
            // CR 702.79a). One test would be satisfied by a second fixed
            // sign.
            "Contextual counters (the ±1/±1 payoff)",
            count(|def| {
                def.all_keywords()
                    .contains(KeywordSet::PERSIST.union(KeywordSet::UNDYING))
            }),
        ),
        (
            "Stack strategy (ward and taxes)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Ward { .. }))),
        ),
        // 35 on 19.09.2026, and two of them only because `Effect::walk`
        // reads behind a price: Flusterstorm and Malevolent Hermit counter
        // a spell *unless its controller pays*, which is a variant carrying
        // one effect rather than a list. A probe matching the top-level
        // list would have reported 33 and named no row for the other two.
        (
            "Stack strategy (counter wars)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::CounterTargetSpell
                        | Effect::CounterTargetSpellToExile
                        | Effect::CounterTargetSpellOrAbility
                        | Effect::CounterTargetAbility
                )
            }),
        ),
        (
            "Stack strategy (copying a spell)",
            count_reaching(|effect| matches!(effect, Effect::CopyTargetSpell { .. })),
        ),
        (
            "Stack strategy (redirecting a target)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::ChangeTarget { .. } | Effect::ChooseNewTargets
                )
            }),
        ),
        // The row's sentence is "scout refresh after shuffle, reveal, wish
        // and sideboarding". A library search is what forces the shuffle, so
        // it is the precondition rather than a neighbouring mechanic — and a
        // wish is the row's own word.
        (
            "Hidden-zone decisions (a library search)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::SearchLibrary { .. } | Effect::OptionalBasicLandSearchFor { .. }
                )
            }),
        ),
        (
            "Hidden-zone decisions (a wish from outside the game)",
            count_reaching(|effect| matches!(effect, Effect::WishToHand { .. })),
        ),
        // "Copy/control/zone changes preserve per-commander damage
        // identity", which needs a card that can change one.
        (
            "Commander identity changes (a controller change)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::ChangeController { .. }
                        | Effect::ExchangeControlOrSacrifice
                        | Effect::ControlRotation
                        | Effect::AllCreaturesToOwner
                )
            }),
        ),
        (
            "Commander identity changes (a token copy of a permanent)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::CreateTokenCopyOf { .. }
                        | Effect::CreateTokenCopyOfEquipped { .. }
                        | Effect::CreateTokenCopyOfFirstToken
                )
            }),
        ),
        // Treasure is asked by identity and not by name: the token table is
        // compiled, so a pointer comparison against `tokens::TREASURE`
        // cannot drift the way a string would.
        //
        // Spent, #223: `a_treasure_pays_for_a_spell_when_nothing_else_can`.
        // The agent read a registry token as printing nothing, so a Treasure
        // was never a mana source.
        (
            "Alternate resource engines (treasure)",
            count_reaching(|effect| {
                // All seven variants that carry a `TokenDef`, and that is
                // the point rather than a flourish: reading the two obvious
                // ones returned 5 and the pool makes 6, the sixth being
                // Fountainport. A probe over a taxonomy is only as honest as
                // the arm it forgot.
                let (Effect::CreateToken { token }
                | Effect::CreateTokenN { token, .. }
                | Effect::CreateTokenPtPerCount { token, .. }
                | Effect::CreateTokenForTargetController { token, .. }
                | Effect::ExileTargetsCreateTokens { token, .. }
                | Effect::Amass { token, .. }
                | Effect::CreateTokenFromLinked { token, .. }) = effect
                else {
                    return false;
                };
                std::ptr::eq(*token, &raw const baylee_cards::tokens::TREASURE)
            }),
        ),
        // Spent, #223: `restricted_mana_pays_for_the_spells_it_names_and_no_\
        // others` through the engine, when the view can read the filter.
        (
            "Alternate resource engines (restricted mana)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::AddMana {
                        restriction: Some(_),
                        ..
                    }
                )
            }),
        ),
        // Taken when free, #223: `a_free_alternative_cost_is_taken_over_the_\
        // printed_one`; a pitch or an evoke keeps the printed cost.
        (
            "Alternate resource engines (an alternative cost)",
            count(|def| {
                def.faces
                    .iter()
                    .any(|face| !face.alternative_costs.is_empty())
            }),
        ),
        // The row's own words are "sacrifice/discard/exile/counter costs
        // with beneficial payoffs", which is an **activation** cost and not
        // a spell's additional one. Asking `additional_costs` returned 3,
        // and exactly one of those three prints a non-mana part at all
        // (Toxic Deluge's `PayLifeX`), so the label named three things the
        // predicate could not see.
        //
        // Asked of the abilities instead there are three numbers and this
        // row asserts on the first: **361 cards**, 382 abilities, 383 parts
        // — 195 `SacrificeSelf`, 85 `Sacrifice`, 71 `DiscardSelf`, 32
        // `Discard`. Fourteen cards pay two different kinds (nine of them
        // the Landscape cycle) and one ability pays two parts, which is the
        // whole of the gap between the three. No exile cost appears at all,
        // which is why the name below does not claim one.
        //
        // The first breakdown written here was taken inside an `any`, so it
        // stopped at each card's first matching ability and undercounted
        // every kind. It summed to 362 against a stated 361 — and a
        // breakdown that does not sum to its own total is how it was
        // caught, for the second time in one day.
        //
        // Refused, #223: `a_card_is_not_given_up_for_a_small_gain`. Another
        // card as the price of a whitelisted gain is not paid; the source
        // paying for itself still is.
        (
            "Alternate resource engines (a sacrifice or discard cost)",
            count(|def| {
                abilities(def).any(|ability| {
                    let (AbilityDef::Activated { cost, .. }
                    | AbilityDef::ActivatedConditional { cost, .. }) = ability
                    else {
                        return false;
                    };
                    cost.parts.iter().any(|part| {
                        matches!(
                            part,
                            CostPart::Sacrifice(_)
                                | CostPart::SacrificeSelf
                                | CostPart::Discard(_)
                                | CostPart::DiscardSelf
                        )
                    })
                })
            }),
        ),
        // `has_variable` and not a spelling: the cost is parsed at compile
        // time, so the question "does this card announce an X" is answered
        // by the type rather than by looking for the letter.
        (
            "Variable costs (X in a printed cost)",
            count(|def| def.faces.iter().any(|face| face.mana_cost.has_variable())),
        ),
        (
            "Variable costs (a cost reduction)",
            count(|def| def.faces.iter().any(|face| face.cost_reduction.is_some())),
        ),
        (
            "Proliferate and replacement effects (the replacement half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Replacement(_)))),
        ),
        // Named for what it measures. **No card in this pool has flashback**;
        // these three give it to somebody else's card (Snapcaster Mage,
        // Emry, Stingcaster Mage). Written as "flashback" the cell promised
        // a card with the keyword and delivered three that hand it out,
        // which is a different test to write.
        (
            "Transformation and alternate zones (flashback granted to another card)",
            count_reaching(|effect| matches!(effect, Effect::GrantFlashback)),
        ),
        // `CopyOnEnter` is a permanent entering *as* a copy — the thirteen
        // clones (24.09.2026), Phyrexian Metamorph through Vesuva — and not
        // an ability copied off another card.
        //
        // Answered for what is copied, #227:
        // `a_clone_copies_what_is_worth_having_twice`.
        (
            "Transformation and alternate zones (enters as a copy)",
            count(|def| {
                abilities(def).any(|a| {
                    matches!(
                        a,
                        AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. }
                    )
                })
            }),
        ),
    ];

    for (row, found) in rows {
        assert!(
            found > 0,
            "no card in the pool reaches `{row}` in docs/ai-coverage-todo.md \
             any more. Either the mechanic left the pool — then move this \
             entry back to the tripwire test above, so its return is \
             noticed — or this probe stopped matching, which is the failure \
             the compiled pool is read for."
        );
    }
}

/// A granted mana ability charges the tap and nothing else, in this pool.
///
/// `simple_mana`'s doc — and `mana_shape` under it — says it reads a
/// **free** mana ability, and what it actually checks is `cost.mana ==
/// ZERO`. Free of *mana* is not free: `cost.parts` may still ask for a life
/// payment, a counter or the permanent itself, and nothing in that family
/// looks. For the AI's planner that was #168, and the fix is a price column
/// in `policy::sources` rather than a change to the reader, because the
/// reader answers what comes *out* of an ability and the price is what goes
/// in.
///
/// For `PublicObject::granted_mana` it is a different question with the same
/// hole, and the answer today is that the hole is unreachable — which is a
/// fact about this pool and not about the code, so it is written here where
/// a card can break it rather than in a comment where it cannot. Every
/// `Modifier::GrantActivated` that grants a *mana* ability costs
/// `Cost::TAP`, so the projection and the offer agree by accident of what is
/// printed. The day one does not, `docs/protocol.md` §"Granted mana" is the
/// paragraph that breaks: a land the planner counts on and the engine
/// refuses.
///
/// The population bound is the load-bearing half. A grant hides in **two**
/// shapes — `AbilityDef::Static`, which `ability_effects` deliberately
/// returns nothing for, and `Effect::CreateContinuousEffect` inside an
/// ordinary effect list — and a walk that found only one of them would
/// report a clean zero over a third of the sites. Chromatic Lantern and
/// Great Divide Guide are statics; Urza's Saga writes both of its through
/// chapters.
#[test]
fn no_granted_mana_ability_charges_more_than_the_tap() {
    let mut sites = 0;
    let mut mana_sites = 0;
    let mut priced: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        let mut modifiers: Vec<&'static Modifier> = Vec::new();
        for ability in abilities(def) {
            if let AbilityDef::Static(statics) = ability {
                modifiers.push(&statics.modifier);
            }
        }
        for effects in abilities(def).flat_map(ability_effects) {
            let mut seen = 0;
            Effect::walk(effects, &mut seen, &mut |effect| {
                if let Effect::CreateContinuousEffect { modifier, .. } = effect {
                    modifiers.push(modifier);
                }
            });
        }
        for modifier in modifiers {
            let Modifier::GrantActivated {
                cost, mana_ability, ..
            } = modifier
            else {
                continue;
            };
            sites += 1;
            if !*mana_ability {
                continue;
            }
            mana_sites += 1;
            if !baylee_cards_dsl::tap_only(cost) {
                priced.push(def.faces[0].name);
            }
        }
    }
    // Four on 20.09.2026 — Chromatic Lantern, Great Divide Guide and both of
    // Urza's Saga's — of which three grant mana. Bounds and not equalities,
    // because a card added is not news and a walk that stopped descending is.
    assert!(
        sites >= 4,
        "the walk found {sites} `GrantActivated` sites and there were four: \
         it is reaching only one of the two shapes a grant is written in"
    );
    assert!(
        mana_sites >= 3,
        "the walk found {mana_sites} granted *mana* abilities and there were \
         three, so the assertion below is over a list too short to fail"
    );
    // What used to stand here was `priced.is_empty()` — no card in the pool
    // may grant a priced mana ability — and Forgotten Monument broke it the
    // day it was written, correctly: the card really does sell its Caves a
    // colour for `{T}` and a life. The hole was never the card's. It was that
    // `GrantedMana` has no field for a price, so `granted_mana` reported the
    // grant as free.
    //
    // `tap_only` is now that rule, read by the projection and asserted here
    // from the other end. This test therefore says the thing that is still
    // worth saying: whatever the pool grants, the predicate the projection
    // uses is the predicate this walk applies, so a card that starts charging
    // for a grant is silently dropped from the view rather than misreported.
    // `view::granted_mana_refuses_a_priced_grant` is the same sentence with a
    // board under it.
    // Named and not counted, so the list falls in both directions: a card
    // that stops charging drops out of it, and a card that starts charging
    // walks into a view that cannot describe it and owes the same board-level
    // test Forgotten Monument has.
    assert_eq!(
        priced,
        ["Forgotten Monument"],
        "the pool's priced granted mana abilities have changed. Each one is \
         dropped from `PublicObject::granted_mana` rather than reported free \
         — see `baylee_cards_dsl::tap_only` — and each owes a test with a \
         board under it, because this walk cannot see a projection"
    );
}

/// **The tripwire fired on 20.09.2026 and this is what is left of it.**
///
/// It used to assert a zero. The counter-clock rows of #73 were answered by
/// two `baylee-ai` unit tests and by no game, because no card in this pool
/// could be *made* to put a lore or a time counter anywhere: both clocks
/// were advanced by the engine itself (`progress.rs`), never by an effect a
/// player activates. Its own message said what the day a card arrived would
/// owe — the same decision reachable in a real game, and that game as a
/// test.
///
/// Trenzalore Clocktower is that card: `{T}: Add {U}. Put a time counter on
/// Trenzalore Clocktower.` **#166 was opened on the prediction that
/// `clock_score` would score it backwards, and that prediction is wrong.**
/// The `Time` arm is written for suspend, and it says so before it reads
/// anything: it asks whether the card underneath prints `Suspend` and
/// answers 0 when it does not, which is the same refusal it already gives
/// vanishing. `a_time_counter_is_not_a_delay_on_a_card_that_is_not_counting_down`
/// pins it with the deciding shape — the Clocktower carrying **three**
/// counters against a suspended card's **four**, so a rule reading the count
/// alone would take the Clocktower and this one does not.
///
/// It is not reached at all, either. The counter is inside the mana ability
/// and has no target, so `targets` — the only caller of `clock_score` — is
/// never consulted about it. Both halves had to be measured, because either
/// one alone would have been the wrong reason.
///
/// **So the list stays, and deleting it is no longer part of closing #166.**
/// The ticket said to delete it once the game was written, on the premise
/// that this card made the decision reachable; it does not, and a tripwire
/// that has not yet caught what it watches for is not one to take down. What
/// it is now waiting for is narrower and worth saying: a lore or time
/// counter effect that **targets**, which is the only shape that reaches the
/// rule. The played game the ticket asked for exists —
/// `a_mana_land_that_also_counts_is_invisible_to_the_planner` in
/// `ai_decisions.rs` — and it pins the real consequence of this card, which
/// is **#170**: the planner cannot read a mana ability that has a second
/// sentence, so the agent never taps this land.
///
/// The zero is a **pinned list** now, the same shape as
/// `LANDS_THAT_WOULD_COUNT_THEMSELVES`: it names what is reachable, it is
/// red the day a second card joins, and it carries the ticket. Relaxing it
/// is not on the table, and neither is adding a name to the list without
/// reading, for that name, the two questions asked above — does anything
/// target it, and what does the arm answer.
///
/// The two assertions above the list are what make it mean something.
/// `signed` proves the walk reaches real `AddCounter` effects at all — the
/// pool is full of +1/+1 — and `seen` is the population it read, so a walker
/// blinded by a refactor fails here instead of reporting a quiet nought.
#[test]
fn only_the_named_cards_put_a_lore_or_time_counter_on_anything() {
    // Named by the card that puts them, so a second arrival is a name in a
    // failure message and not a number to bump.
    const REACHABLE_CLOCKS: &[&str] = &["Trenzalore Clocktower"];
    let mut seen = 0;
    let mut kinds = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            counters_put(effects, &mut seen, &mut kinds);
        }
    }
    let mut clocks: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            let mut here = 0;
            let mut mine = Vec::new();
            counters_put(effects, &mut here, &mut mine);
            if mine
                .iter()
                .any(|kind| matches!(kind, CounterKind::Lore | CounterKind::Time))
            {
                clocks.push(def.faces[0].name);
            }
        }
    }
    clocks.sort_unstable();
    clocks.dedup();
    let signed = kinds
        .iter()
        .filter(|kind| matches!(kind, CounterKind::Plus { .. } | CounterKind::Minus { .. }))
        .count();

    // 2731 on 19.09.2026. The floor is the population and not the number,
    // so a card added or removed is not news and a walker that stopped
    // descending is.
    assert!(
        seen > 2_000,
        "the effect walk visited {seen} effects, which is too few to have \
         read this pool: the nesting shapes it follows have gone stale"
    );
    assert!(
        signed > 0,
        "the walk found no +1/+1 or -1/-1 counter in the whole pool, so it \
         is not reaching `AddCounter` at all and the zero below is empty"
    );
    assert_eq!(
        clocks, REACHABLE_CLOCKS,
        "the set of pool cards that put a lore or time counter on something \
         has changed. `tactics::clock_score` is reachable in a real game \
         through each of them and #166 owes that game as a test — so a new \
         name here is a game to write, not a list to extend."
    );
}

/// The pool really does hide effects behind "unless you pay", and this is
/// the population that says so.
///
/// [`Effect::PlayerMayPayOr`] and [`Effect::PlayerMayPayCostOr`] carry a
/// single `&'static Effect` rather than a list, and every hand-rolled walker
/// in this workspace descended into the lists and stopped there. That is a
/// blind spot no reader reported, because an effect nobody walks looks
/// exactly like an effect that is not there — so this test asks the pool
/// what is behind the clause instead of asking a reader whether it looked.
///
/// Measured on 19.09.2026: **35 effects on 35 cards**, being 29
/// `SacrificeSelf` (every Karoo land and the upkeep creatures), 3
/// `DrawCards` (Esper Sentinel, Mystic Remora, Rhystic Study), 2
/// `CounterTargetSpell` (Flusterstorm, Malevolent Hermit) and 1
/// `CreateToken` (Smothering Tithe). Floors and not the numbers: a card
/// joining or leaving is not news, and a walk that stopped descending is.
///
/// The breakdown is the test's own, taken by tallying `behind` rather than
/// by grepping card files: the first reading of it said 29 and then wrote
/// 28, and 28 + 3 + 2 + 1 is 34 against a total of 35. A census whose parts
/// do not sum to its whole has a card in it nobody looked at.
#[test]
fn the_pool_hides_effects_behind_a_price() {
    let (all, seen) = pool_effects();
    let behind: Vec<&'static Effect> = all
        .iter()
        .flat_map(|effect| match effect {
            Effect::PlayerMayPayOr { .. } | Effect::PlayerMayPayCostOr { .. } => {
                effect.branches().0
            }
            _ => &[][..],
        })
        .collect();
    let sacrifices = behind
        .iter()
        .filter(|effect| matches!(effect, Effect::SacrificeSelf))
        .count();

    assert!(
        seen > 2_000,
        "the effect walk visited {seen} effects, which is too few to have \
         read this pool: the nesting shapes it follows have gone stale"
    );
    assert!(
        behind.len() >= 20,
        "the walk found {} effect(s) behind a price where 35 were measured. \
         Either the pool stopped printing the clause or `Effect::branches` \
         stopped descending into it — and the second is the one that makes \
         every reader in this workspace quietly short.",
        behind.len()
    );
    assert!(
        sacrifices >= 15,
        "{sacrifices} of the effects behind a price are `SacrificeSelf` \
         where 28 were measured. The Karoo lands are what that clause is \
         mostly made of here, so this reaching nought means the walk found \
         the wrapper and not what is inside it"
    );
}

/// A face states at most one *list* of modes.
///
/// The question the agent answers names no ability: a modal trigger asks
/// through the permanent and a mode number, and which of its abilities is on
/// the stack lives in the engine's trigger queue. So a face with two modal
/// abilities over two different mode lists is two lists and one number, and
/// `baylee_ai::filter::modal_modes` refuses it and falls back to the printed
/// order rather than picking a list.
///
/// What the pool prints is the harmless half of that, and it is why the rule
/// is about lists and not abilities: Derevi, Empyrial Tactician is one
/// printed sentence with two trigger conditions ("when this enters **and**
/// whenever a creature you control deals combat damage to a player"), written
/// as two `modal_triggered!` over the same `TAP_OR_UNTAP` modes. Naming that
/// shared list is unambiguous however the trigger arrived.
///
/// It asks through `abilities_for_face`, which is the lookup the agent uses
/// and not the union `abilities` builds: face 0 falls back to the card-level
/// list only when it states none of its own, and a probe reading both at once
/// would find two lists where the card has one.
///
/// Two injected findings, because the assertion is an emptiness. `modal`
/// proves the walk reaches modal abilities at all; `shared` proves it reaches
/// the two-abilities-one-list case, which is the case the assertion is a
/// statement about and would otherwise be indistinguishable from a pool where
/// every face has exactly one.
#[test]
fn no_pool_face_states_two_mode_lists() {
    let mut faces = 0;
    let mut modal = 0;
    let mut shared = 0;
    let mut split: Vec<&'static str> = Vec::new();
    for def in baylee_cards::all() {
        for (index, face) in def.faces.iter().enumerate() {
            faces += 1;
            let mut lists: Vec<&'static [baylee_cards::dsl::SpellMode]> = Vec::new();
            let mut here = 0;
            for ability in def.abilities_for_face(index) {
                let (AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. }) =
                    ability
                else {
                    continue;
                };
                here += 1;
                if !lists.iter().any(|seen| std::ptr::eq(*seen, *modes)) {
                    lists.push(modes);
                }
            }
            if here > 0 {
                modal += 1;
            }
            if here > lists.len() {
                shared += 1;
            }
            if lists.len() > 1 {
                split.push(face.name);
            }
        }
    }

    // 2836 faces over 2716 cards on 19.09.2026, 10 of them modal and one —
    // Derevi — stating one list twice. The floor is the population and not
    // the number: a card added or removed is not news, a walk that stopped
    // descending into faces is.
    //
    // The 10 is also why the walk is compiled rather than textual, and it is
    // wrong in both directions. Grepping the card files for `ModalSpell` and
    // `ModalTriggered` finds **four**, because seven more are written with
    // the `modal_triggered!` macro and never spell a variant at all — and
    // grepping for the macro as well finds **eleven**, because Marionette
    // Apprentice names it in a `// NOT SUPPORTED:` comment about the
    // fabricate clause it does *not* have.
    assert!(
        faces > 2_500,
        "the walk visited {faces} faces, which is too few to have read this \
         pool: it has stopped descending into the cards it is given"
    );
    assert!(
        modal > 0,
        "the walk found no modal ability in the whole pool, so it is not \
         reaching `ModalSpell` at all and the emptiness below is empty"
    );
    assert!(
        shared > 0,
        "no face in the pool states one mode list twice, so this test no \
         longer distinguishes `modal_modes`'s rule from the stricter one it \
         replaced: check that Derevi still writes its two triggers over one \
         `TAP_OR_UNTAP` before weakening this"
    );
    assert!(
        split.is_empty(),
        "{split:?} state two different mode lists on one face. \
         `baylee_ai::filter::modal_modes` refuses such a face and the agent \
         takes the printed order there: give the mode list an ability handle, \
         or record here why the printed order is the right answer for it."
    );
}

/// #76: the land the agent would miss if it read only the face that is up.
///
/// A modal double-faced card is filed under its front, and the front is the
/// spell — so `HandObject::types` says sorcery and the land is on the other
/// side. The engine does not read it that way: `compute_legal` offers the
/// land drop for a modal card's land back (CR 712.12) and not for a
/// transforming one's (CR 712.8a, #152). `policy::plays_as_land` asks the
/// same function, and this is the population it is worth having: a floor
/// rather than the number, because a card joining the pool does not make
/// the rule less true, and nought would mean the rule is being kept for
/// nothing.
#[test]
fn the_pool_prints_lands_on_a_back_face() {
    let hidden = baylee_cards::all()
        .filter(|def| {
            def.faces
                .iter()
                .skip(1)
                .any(|f| f.types.contains(TypeSet::LAND) && f.castable_from_hand)
                && !def.faces[0].types.contains(TypeSet::LAND)
        })
        .count();
    let front = baylee_cards::all()
        .filter(|def| def.faces[0].types.contains(TypeSet::LAND))
        .count();
    assert!(
        hidden >= 45,
        "{hidden} modal card(s) print a land on a face that is not the front, \
         where 50 were measured on 24.09.2026 (82 with the transforming ones). \
         Below this the rule `plays_as_land` states is being kept for a \
         handful of cards and is worth re-reading."
    );
    assert!(
        front > hidden,
        "{front} card(s) print a land on the front against {hidden} behind \
         one, which is the wrong way round for this pool: the front-face \
         reading would then be the special case, not the rule."
    );
}
