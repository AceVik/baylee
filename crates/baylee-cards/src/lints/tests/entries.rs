//! Loyalty, entering, waterbending, counters, stations and returns.

use super::*;

/// A loyalty ability belongs to a planeswalker, and a planeswalker says
/// what it starts on.
///
/// Per **card** for the first half and per face for the second: face 0
/// shadows the card-level ability list, so a walker's `+1` is normally
/// written beside the faces rather than inside one, and Jace, Vryn's
/// Prodigy is a creature on its front and a planeswalker on its back.
///
/// Seven planeswalker faces in the pool, measured 2026-09-09 — the same
/// seven `xtask validate` holds against their printed starting loyalty.
#[test]
fn only_a_planeswalker_has_loyalty() {
    let mut wrong = Vec::new();
    for def in crate::all() {
        let walker = def
            .faces
            .iter()
            .any(|face| face.types.contains(TypeSet::PLANESWALKER));
        let loyalty_ability = def
            .abilities
            .iter()
            .chain(def.faces.iter().flat_map(|face| face.abilities))
            .any(|ability| matches!(ability, AbilityDef::Loyalty { .. }));
        if loyalty_ability && !walker {
            wrong.push(format!(
                "{} has a loyalty ability and no walker face",
                def.name()
            ));
        }
        for face in def.faces {
            if loyalty_fault(face) {
                wrong.push(format!(
                    "{} ({}) is a planeswalker with no starting loyalty",
                    def.name(),
                    face.name
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} card(s) disagree with CR 306.5b about loyalty.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// No card file spells an enter-trigger out; they all say `Trigger::ETB`.
///
/// The pool wrote `Trigger::EntersBattlefield(&Filter::This)` ninety-nine
/// times while the constant that *is* those bytes had nought uses, so
/// this is the shape that was just swept and the guard that keeps it
/// swept. Sixty-six of the ninety-nine were the transcoder's output and
/// are held by the emitter as well; the other thirty-three are held by
/// nothing else.
///
/// Whitespace is collapsed before matching, because rustfmt wraps a long
/// call and a reader that matched the unwrapped spelling would read a
/// wrapped card as clean — which is how seven textual readers of this
/// pool have already been blind.
///
/// An enter-trigger pointing at something *other* than the source keeps
/// the variant and its filter: eleven of the pool's hundred and ten do,
/// and this says nothing about them.
#[test]
fn no_card_writes_an_enter_trigger_out_by_hand() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut offenders = Vec::new();
    let mut stack = vec![root.clone()];
    let mut files = 0usize;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("the card tree is readable") {
            let path = entry.expect("a readable directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files += 1;
                let text = std::fs::read_to_string(&path).expect("a readable card file");
                let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if flat.contains("Trigger::EntersBattlefield( &Filter::This")
                    || flat.contains("Trigger::EntersBattlefield(&Filter::This")
                {
                    offenders.push(
                        path.strip_prefix(&root)
                            .unwrap_or(&path)
                            .display()
                            .to_string(),
                    );
                }
            }
        }
    }

    assert!(
        files > 1000,
        "walked {files} card files — the walk is broken, not the pool"
    );
    assert!(
        offenders.is_empty(),
        "{} card(s) spell an enter-trigger out where `Trigger::ETB` is \
         the same bytes and the word said at a table:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// A modifier that *asks* is the last thing an entry does, and the engine
/// can only do that once.
///
/// `apply_enter_modifiers` publishes a `Pending` and returns, having
/// already advanced past this arrival — so a second question would be
/// dropped in the same silence that used to swallow `Tapped` behind
/// `ChooseColor`. Asking them one after the other means suspending the
/// scan, which is a rule nobody needs yet: no card in the pool prints two.
///
/// That is a claim about a population, so it is a test rather than a
/// comment. The day a card prints "as this enters, choose a color and
/// choose a creature type", this fails and hands a person the example.
#[test]
fn no_face_asks_two_questions_as_it_enters() {
    use baylee_cards_dsl::EnterModifier;

    // An exhaustive `match` and not a `matches!` list, which is the
    // difference between a lint that grows with the DSL and one that
    // quietly stops covering it. `TappedUnlessReveal` was added later
    // and asks — under a positive list, eighteen lands would have
    // entered the pool carrying a question this sweep could not see.
    let asks = |m: &EnterModifier| match m {
        EnterModifier::ChooseSubtype
        | EnterModifier::ChooseBasicLandType
        | EnterModifier::ChooseCardName
        | EnterModifier::ChooseColor
        | EnterModifier::ChooseOpponent
        | EnterModifier::ChooseColorExcept(_)
        | EnterModifier::TappedOrPayLife(_)
        | EnterModifier::TappedUnlessReveal(_) => true,
        EnterModifier::Tapped
        | EnterModifier::TappedUnless(_)
        | EnterModifier::TappedUnlessCount { .. }
        | EnterModifier::TappedUnlessAtMost { .. }
        | EnterModifier::TappedUnlessOpponents { .. }
        | EnterModifier::TappedUnlessSomeoneAtOrBelow { .. }
        | EnterModifier::Prepared
        | EnterModifier::LoseLifeEqualToLife
        | EnterModifier::WithCounters { .. } => false,
    };
    let mut offenders = Vec::new();
    let mut asking = 0_usize;
    for def in crate::all() {
        for face in def.faces {
            let found: Vec<&EnterModifier> =
                face.enter_modifiers.iter().filter(|m| asks(m)).collect();
            asking += found.len();
            if found.len() > 1 {
                offenders.push(format!("{}: {found:?}", face.name));
            }
        }
    }
    assert!(
        asking > 20,
        "only {asking} asking enter-modifier(s) found — the walk has gone \
         blind, and an empty sweep proves nothing"
    );
    assert!(
        offenders.is_empty(),
        "{} face(s) ask more than one question as they enter. The entry \
         scan answers one and returns, so the rest are dropped without a \
         word — teach `apply_enter_modifiers` to suspend before adding \
         the card.\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// An entry clause says "you control", and the filter has to say it
/// too, because `controls_count` does not.
///
/// Both shapes: the two **counted** variants and the single-permanent
/// `EnterModifier::TappedUnless`. The second was left out once, on the
/// premise that the Turbulent cycle asks about an *opponent*'s Swamp —
/// but that cycle is `TappedUnlessCount` with `ControlledByOpponent`,
/// which `scoped` accepts, and every `TappedUnless` sentence in the pool
/// prints "you control". Five of them were unscoped when it was added
/// (Reef Roads, Rocky Roads, Chocobo Camp, Spymaster's Vault, Cori
/// Mountain Monastery): an opponent's Vehicle, legend or basic let each
/// enter untapped.
///
/// `Engine::controls_count` walks the **whole** battlefield and applies
/// the filter with the entering permanent's controller as the "you". So
/// `Filter::LAND` counts the opponent's lands and `Filter::YOUR_LAND`
/// does not, and every one of these 35 cards prints "you control".
/// Spelled unscoped, a fastland turns off and a slowland turns on
/// because of lands across the table — a card that is wrong in every
/// real game and right in every test with one player's board on it.
///
/// Two cards were written that way by a card lane on 20.09.2026 (Thran
/// Portal, Hall of Storm Giants) against thirty-three spelled
/// correctly, which is why this is a lint and not a note: the majority
/// being right is what makes the minority invisible.
#[test]
fn every_entry_clause_is_scoped_to_its_controller() {
    use baylee_cards_dsl::{EnterModifier, Filter};

    // The filter as written, for the three modifiers. A `Filter`
    // has no name at runtime, so the question is asked of the shape:
    // anything that is not an `And` containing `ControlledByYou` counts
    // every permanent on the battlefield.
    fn scoped(f: &Filter) -> bool {
        match f {
            Filter::ControlledByYou | Filter::ControlledByOpponent => true,
            // One scoping part is enough inside an `And`; inside an `Or`
            // every branch has to carry one, or the unscoped branch is
            // the one that counts the table.
            Filter::And(parts) => parts.iter().any(scoped),
            Filter::Or(parts) => parts.iter().all(scoped),
            _ => false,
        }
    }

    let mut offenders = Vec::new();
    let (mut counted, mut single) = (0_usize, 0_usize);
    for def in crate::all() {
        for face in def.faces {
            for m in face.enter_modifiers {
                let filter = match m {
                    EnterModifier::TappedUnlessCount { filter, .. }
                    | EnterModifier::TappedUnlessAtMost { filter, .. } => {
                        counted += 1;
                        *filter
                    }
                    EnterModifier::TappedUnless(filter) => {
                        single += 1;
                        *filter
                    }
                    _ => continue,
                };
                if !scoped(filter) {
                    offenders.push(format!("{}: {filter:?}", face.name));
                }
            }
        }
    }
    // One floor per door, so a matcher that stops seeing one shape
    // cannot pass on the other's population.
    assert!(
        counted > 30 && single > 40,
        "only {counted} counted and {single} single entry clause(s) found \
         — the walk has gone blind, and an empty sweep proves nothing"
    );
    assert!(
        offenders.is_empty(),
        "{} entry clause(s) look at every permanent on the battlefield \
         and not the controller's. `controls_count` scopes nothing, so \
         the filter must: use `Filter::YOUR_LAND`, `f!(your …)` or an \
         `And` carrying `ControlledByYou` wherever the card prints \
         \"you control\".\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// Each shape the waterbend lint refuses, and the one it lets through.
#[test]
fn the_waterbend_lint_refuses_a_face_the_wizard_would_misread() {
    const SIX: &[Cost] = &[Cost {
        mana: ManaCost::parse("{6}"),
        parts: &[],
    }];
    const BLUE: &[Cost] = &[Cost {
        mana: ManaCost::parse("{5}{U}"),
        parts: &[],
    }];
    const SACRIFICE: &[Cost] = &[Cost {
        mana: ManaCost::parse("{6}"),
        parts: &[CostPart::SacrificeSelf],
    }];
    let face = |additional_costs, convoke| FaceDef {
        additional_costs,
        convoke,
        waterbend: true,
        ..FaceDef::DEFAULT
    };
    assert_eq!(waterbend_fault(&face(SIX, false)), None);
    for (broken, why) in [
        (face(SIX, true), "convoke beside it"),
        (face(&[], false), "no cost"),
        (face(BLUE, false), "a coloured pip"),
        (face(SACRIFICE, false), "a non-mana part"),
    ] {
        assert!(waterbend_fault(&broken).is_some(), "{why} went through");
    }
}

/// Spirit Water Revival is the pool's one waterbend, measured 2026-09-24.
#[test]
fn every_waterbend_in_the_pool_is_one_the_wizard_can_ask() {
    let mut seen = 0_usize;
    let mut wrong = Vec::new();
    for def in crate::all() {
        for face in def.faces.iter().filter(|f| f.waterbend) {
            seen += 1;
            if let Some(fault) = waterbend_fault(face) {
                wrong.push(format!("{} ({}) {fault}", def.name(), face.name));
            }
        }
    }
    assert!(
        (1..=10).contains(&seen),
        "{seen} waterbend faces; the sweep read something other than the pool's"
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The `CounterKind::Custom` ids a compiled definition carries, read off
/// its `Debug` rendering — the one `xtask pool-dump` writes.
///
/// Read that way and not by walking the places a `CounterKind` can sit
/// (an entry modifier, a cost, an effect inside a branch, a condition, a
/// modifier, a granted ability, a token) because a walk names the doors
/// it knows and goes silent on the next one, where the rendering has
/// every field in it. `CounterKind::Custom` is the one variant named
/// `Custom` a definition can hold, so `Custom(` and digits is an id.
fn custom_counter_ids(rendered: &str) -> Vec<u16> {
    rendered
        .match_indices("Custom(")
        .filter_map(|(at, open)| {
            rendered[at + open.len()..]
                .split(')')
                .next()
                .and_then(|digits| digits.parse().ok())
        })
        .collect()
}

/// **Every custom counter the pool carries is an id
/// `counters::ASSIGNED` gives a word to.**
///
/// Wishclaw Talisman's wish counters and Eumidian Hatchery's hatchling
/// counters were both `CounterKind::Custom(5)`, each behind a constant
/// local to its card, while the registry stopped at 4: two words, one
/// counter, and nothing that could see it — a proliferate or a "remove
/// a counter" would have treated each as the other. An id the registry
/// does not name is a number some other card can take next, so this
/// fails on one; `no_card_file_spells_a_counter_id_as_a_number` below is
/// the other half, and `counters`' own `no_two_counter_words_share_an_id`
/// keeps the registry's ids apart.
#[test]
fn every_custom_counter_in_the_pool_is_an_assigned_id() {
    use crate::dsl::counters::ASSIGNED;
    let assigned: Vec<u16> = ASSIGNED
        .iter()
        .filter_map(|(_, kind)| match kind {
            crate::dsl::CounterKind::Custom(id) => Some(*id),
            _ => None,
        })
        .collect();
    let mut carriers = 0_usize;
    let mut unassigned = Vec::new();
    let rendered = crate::all()
        .map(|def| (def.name(), format!("{def:?}")))
        .chain(
            crate::tokens::ALL
                .iter()
                .map(|token| (token.name, format!("{token:?}"))),
        );
    for (name, text) in rendered {
        let ids = custom_counter_ids(&text);
        carriers += usize::from(!ids.is_empty());
        for id in ids {
            if !assigned.contains(&id) {
                unassigned.push(format!("{name} carries CounterKind::Custom({id})"));
            }
        }
    }
    unassigned.sort();
    unassigned.dedup();
    assert!(
        unassigned.is_empty(),
        "a custom counter id no word in `counters::ASSIGNED` owns — assign \
         the printed word an id there and write the constant:\n{}",
        unassigned.join("\n")
    );
    // Measured 2026-09-24: 31 definitions carry one (the seventeen
    // storage lands, ten depletion lands, Gemstone Mine, Luminarch
    // Ascension, Wishclaw Talisman, Eumidian Hatchery). The floor is what
    // keeps a changed `Debug` spelling from reading as a clean pool; the
    // ceiling, twice that, is what keeps something that is not a counter
    // from being read as one, and is the number to raise when the pool
    // honestly outgrows it.
    assert!(
        (31..=62).contains(&carriers),
        "{carriers} definitions carry a custom counter; the reader is not \
         reading the pool's counters"
    );
}

/// **No card file writes a counter id as a number.** It names the
/// constant `baylee_cards_dsl::counters` assigns (`counters::STORAGE`),
/// which the prelude carries.
///
/// The half the compiled pool cannot see: a card that wrote
/// `CounterKind::Custom(5)` for a word of its own would compile to the
/// same value as `counters::WISH` and pass the sweep above, while being
/// a second word on one id — which is how Wishclaw Talisman and
/// Eumidian Hatchery came to share one. Comments are skipped, since the
/// cards that could not yet name a word say so in them.
#[test]
fn no_card_file_spells_a_counter_id_as_a_number() {
    let mut offenders = Vec::new();
    let mut naming = 0_usize;
    for (name, text) in crate::tests::every_card_file() {
        let code: String = text
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ");
        if code.contains("Custom(") {
            offenders.push(name);
        } else if code.contains("counters::") {
            naming += 1;
        }
    }
    assert!(
        offenders.is_empty(),
        "these card files write a `CounterKind::Custom` id by number; \
         assign the word in `baylee_cards_dsl::counters` and name the \
         constant: {offenders:?}"
    );
    // Measured 2026-09-24: 34 files say `counters::` outside a comment
    // — the carriers above, plus the few whose coverage reason names a
    // constant that does not exist yet. Floor and ceiling for the reason
    // the sweep above has them: a stripper that took everything would
    // find nothing, and one that took nothing would count the comments.
    assert!(
        (34..=68).contains(&naming),
        "{naming} card files say `counters::` outside a comment; the \
         reader is not reading the pool's cards"
    );
}

/// Station (CR 702.184a) means "Tap another untapped creature you
/// control: Put a number of charge counters on this permanent equal to
/// the tapped creature's power. Activate only as a sorcery." Every
/// ability whose printed sentence is a station is that and nothing else:
/// the creature is a cost (`CostPart::TapOther`, where CR 118.3 supplies
/// "untapped"), nothing is targeted, the count is the tapped creature's
/// power (`Amount::TappedPower`), and the timing is a sorcery's.
/// Inspirit, Flagship Vessel and U.S.S. Enterprise-D wrote it as
/// `Effect::TapTarget` aimed at "another creature you control" under a
/// free cost, which offered a tapped creature to "tap".
///
/// The count has a floor and a ceiling: three stations written, over
/// seven station cards in the pool, counted 2026-09-30.
#[test]
fn every_station_is_the_ability_its_keyword_spells() {
    use crate::dsl::ability::ActivationTiming;
    use crate::dsl::effect::Amount;
    let mut wrong = Vec::new();
    let mut checked = 0_usize;
    for def in crate::all() {
        for face in 0..def.faces.len() {
            for (index, ability) in def.abilities_for_face(face).iter().enumerate() {
                let Some(text) = u32::try_from(index)
                    .ok()
                    .and_then(|index| crate::lines::ability_line(def.index, face, index))
                    .and_then(|at| crate::oracle::sentence(def.index, face, at.line))
                else {
                    continue;
                };
                if !text.starts_with("Station (") {
                    continue;
                }
                checked += 1;
                let who = format!("{} face {face} ability {index}", def.name());
                let (AbilityDef::Activated {
                    cost,
                    effects,
                    targets,
                    second_targets,
                    timing,
                    ..
                }
                | AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    targets,
                    second_targets,
                    timing,
                    ..
                }) = ability
                else {
                    wrong.push(format!("{who}: not an activated ability"));
                    continue;
                };
                let taps = cost.mana == ManaCost::ZERO
                    && matches!(
                        cost.parts,
                        [CostPart::TapOther(filter)]
                            if **filter == Filter::ANOTHER_CREATURE_YOU_CONTROL
                    );
                let counts = matches!(
                    effects,
                    [Effect::AddCounter {
                        kind: crate::dsl::effect::CounterKind::Charge,
                        amount: Amount::TappedPower,
                    }]
                );
                if !taps
                    || !counts
                    || targets.is_some()
                    || second_targets.is_some()
                    || *timing != ActivationTiming::SorcerySpeed
                {
                    wrong.push(format!(
                        "{who}: cost {cost:?}, effects {effects:?}, targets {targets:?}, \
                         timing {timing:?}"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert!(
        (3..=7).contains(&checked),
        "read {checked} stations out of the pool, and three were written"
    );
}

/// Where `Effect::ExileSelfReturnAsFace` puts the card back is its
/// printed sentence's answer: `owner_control` exactly where the sentence
/// says "under its owner's control" (Sheoldred's `{4}{B}`, the Ojers'
/// dies triggers). "…under your control" says the other thing; a
/// sentence that names nobody (The True Scriptures III) puts the card
/// under the player the effect instructs (CR 110.2a). The effect returned
/// every card under its owner's control until observed fault 62, and no
/// card said which it meant.
///
/// And the sentence says "return": one that only says "transform this"
/// is `Effect::TransformSource`, which turns the same permanent over
/// (CR 701.27a, CR 712.18). Eight cards wrote that as an exile and a
/// return (#206), a new object that entered and shed every effect on the
/// old one, until 2026-09-30.
///
/// A printed ability with no known sentence is a finding, not a skip,
/// and the count has a floor and a ceiling: seven, over six cards,
/// counted 2026-09-30 (fifteen over thirteen before the eight stand-ins
/// became transforms).
#[test]
fn every_self_return_comes_back_under_the_control_its_sentence_prints() {
    let mut wrong = Vec::new();
    let mut checked = 0_usize;
    for def in crate::all() {
        for face in 0..def.faces.len() {
            for (index, ability) in def.abilities_for_face(face).iter().enumerate() {
                for (effects, _, door) in resolving_lists(ability) {
                    let mut seen = 0_usize;
                    Effect::walk(effects, &mut seen, &mut |effect| {
                        let Effect::ExileSelfReturnAsFace { owner_control, .. } = effect else {
                            return;
                        };
                        checked += 1;
                        let who = format!("{} face {face} ability {index}", def.name());
                        let sentence = u32::try_from(index)
                            .ok()
                            .filter(|_| matches!(door, Door::Printed))
                            .and_then(|index| crate::lines::ability_line(def.index, face, index))
                            .and_then(|at| crate::oracle::sentence(def.index, face, at.line));
                        match sentence {
                            None => {
                                wrong.push(format!("{who}: no printed sentence ({door:?})"));
                            }
                            Some(text) => {
                                let printed = text.contains("under its owner's control");
                                if printed != *owner_control {
                                    wrong.push(format!(
                                        "{who}: owner_control {owner_control}, prints {text:?}"
                                    ));
                                }
                                if !text.contains("return") {
                                    wrong.push(format!(
                                        "{who}: prints no return, so it is \
                                         Effect::TransformSource: {text:?}"
                                    ));
                                }
                            }
                        }
                    });
                }
            }
        }
    }
    for token in crate::tokens::ALL {
        for ability in token.abilities {
            for (effects, ..) in resolving_lists(ability) {
                let mut seen = 0_usize;
                Effect::walk(effects, &mut seen, &mut |effect| {
                    if matches!(effect, Effect::ExileSelfReturnAsFace { .. }) {
                        wrong.push(format!("token {}: has no face to return as", token.name));
                    }
                });
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert!(
        (7..=14).contains(&checked),
        "read {checked} self-returns out of the pool, and seven were written"
    );
}
