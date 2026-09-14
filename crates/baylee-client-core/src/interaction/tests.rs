use super::*;

// ------------------------------------------------- how a game ends

fn ended(winner: Option<Victor>, reason: EndReason) -> GameResult {
    GameResult { winner, reason }
}

#[test]
fn the_seat_that_won_and_the_seat_that_did_not_read_different_lines() {
    let result = ended(
        Some(Victor::Player(PlayerId::new(0))),
        EndReason::LastPlayerStanding,
    );
    let mine = verdict(Lang::En, &result, PlayerId::new(0), None);
    let theirs = verdict(Lang::En, &result, PlayerId::new(1), None);
    assert_ne!(mine, theirs);
    assert_eq!(mine, Phrase::YouWon.text(Lang::En));
    assert_eq!(theirs, Phrase::YouLost.text(Lang::En));
}

#[test]
fn a_team_wins_for_everyone_sitting_on_it() {
    let result = ended(Some(Victor::Team(2)), EndReason::LastTeamStanding);
    let ours = verdict(Lang::En, &result, PlayerId::new(3), Some(2));
    let theirs = verdict(Lang::En, &result, PlayerId::new(1), Some(1));
    // The seat that won is on the team, not the one the engine named.
    assert_eq!(ours, Phrase::YourTeamWon.fill(Lang::En, &["2"]));
    assert_eq!(theirs, Phrase::TheirTeamWon.fill(Lang::En, &["2"]));
}

#[test]
fn a_verdict_is_a_headline_and_is_written_like_one() {
    // Every other line the prompt bar shows starts with a capital; these
    // five were the outliers, and the end screen sets them at 44 px.
    for lang in Lang::ALL {
        for line in [
            verdict(lang, &ended(None, EndReason::Draw), me(), None),
            verdict(
                lang,
                &ended(Some(Victor::Player(me())), EndReason::EffectWin),
                me(),
                None,
            ),
            verdict(
                lang,
                &ended(Some(Victor::Player(PlayerId::new(9))), EndReason::EffectWin),
                me(),
                None,
            ),
            verdict(
                lang,
                &ended(Some(Victor::Team(1)), EndReason::LastTeamStanding),
                me(),
                Some(1),
            ),
        ] {
            let first = line.chars().next().expect("a verdict is never empty");
            assert!(first.is_uppercase(), "{lang:?}: {line:?}");
        }
    }
}

#[test]
fn a_draw_is_the_one_ending_that_gets_no_second_line() {
    for lang in Lang::ALL {
        assert_eq!(ending_reason(lang, &ended(None, EndReason::Draw)), None);
        for reason in [
            EndReason::LastPlayerStanding,
            EndReason::LastTeamStanding,
            EndReason::EffectWin,
        ] {
            let line =
                ending_reason(lang, &ended(None, reason)).expect("every other ending says how");
            assert!(!line.is_empty());
        }
    }
}

#[test]
fn the_reason_never_takes_a_side() {
    // The winner and the loser read the same second line, so it may not
    // be written from either chair: one text per reason, per language.
    for lang in Lang::ALL {
        let mut seen: Vec<String> = Vec::new();
        for reason in [
            EndReason::LastPlayerStanding,
            EndReason::LastTeamStanding,
            EndReason::EffectWin,
        ] {
            let line = ending_reason(lang, &ended(Some(Victor::Player(me())), reason))
                .expect("every other ending says how");
            let other = ending_reason(lang, &ended(Some(Victor::Player(PlayerId::new(9))), reason))
                .expect("every other ending says how");
            assert_eq!(line, other, "{lang:?} {reason:?}");
            assert!(!seen.contains(&line), "two reasons share a line: {line:?}");
            seen.push(line);
        }
    }
}

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// A seat as a defender.
fn seat(id: u8) -> Defender {
    Defender::Player(PlayerId::new(id))
}

/// The attacker choice with a given list of legal attackers and defenders.
fn attack_choice(attackers: Vec<ObjectId>, defenders: Vec<Defender>) -> Pending {
    Pending::ChooseAttackers {
        player: me(),
        attackers,
        defenders,
    }
}

fn interaction(pending: Pending) -> Interaction {
    Interaction::new(pending, me())
}

#[test]
fn a_choice_addressed_to_another_seat_is_not_actionable() {
    let mut i = interaction(Pending::ChooseTargets {
        player: PlayerId::new(1),
        options: vec![obj(1)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert!(!i.is_mine());
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Rejected);
    assert!(i.confirm().is_none());
    assert!(matches!(i.prompt(), Prompt::Waiting { on: Some(_) }));
}

#[test]
fn only_offered_targets_can_be_selected() {
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    // Not in the offered set: the client refuses to even express it.
    assert_eq!(i.toggle(obj(99)), SelectionOutcome::Rejected);
    assert_eq!(i.selected().collect::<Vec<_>>(), vec![obj(1)]);
}

#[test]
fn the_maximum_is_enforced_and_toggling_off_frees_a_slot() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2), obj(3)],
        min: 1,
        max: 2,
        prompt: ChoicePrompt::Generic,
    });
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Full);
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Removed);
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Added);
}

/// The keyboard's half of a list of cards: one key walks the offer, the
/// other acts where it stopped. Without the second there is no way to
/// answer a search from the keyboard at all — which is how the confirm
/// key came to be doing it, and how a `min: 0` search came to be
/// answered with "nothing" by a player who was passing priority.
#[test]
fn the_focus_keys_can_build_a_whole_answer_on_their_own() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2), obj(3)],
        min: 0,
        max: 2,
        prompt: ChoicePrompt::Generic,
    });
    // The focus starts on the first option, so this needs no walk.
    assert_eq!(i.aim(), Some(Pick::Object(obj(1))));
    assert_eq!(i.toggle_focused(), SelectionOutcome::Added);
    assert_eq!(i.selected().collect::<Vec<_>>(), vec![obj(1)]);
    // And it takes back where it ticked, which is what makes a stray
    // press harmless: the same key on the same row undoes it.
    assert_eq!(i.toggle_focused(), SelectionOutcome::Removed);
    assert!(i.selected().next().is_none());
    // Walk, then tick: the pair reaches any row in the list.
    i.cycle_focus(2);
    assert_eq!(i.aim(), Some(Pick::Object(obj(3))));
    assert_eq!(i.toggle_focused(), SelectionOutcome::Added);
    assert_eq!(i.selected().collect::<Vec<_>>(), vec![obj(3)]);
}

/// Every mode that has no focus has nothing to tick, and says so rather
/// than reaching for whatever happens to be first.
#[test]
fn a_question_with_no_focus_ticks_nothing() {
    let mut i = interaction(Pending::Mulligan {
        player: me(),
        taken: 0,
        next_is_free: true,
    });
    assert_eq!(i.aim(), None);
    assert_eq!(i.toggle_focused(), SelectionOutcome::Rejected);
}

#[test]
fn a_minimum_blocks_confirmation_until_it_is_met() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2)],
        min: 2,
        max: 2,
        prompt: ChoicePrompt::Generic,
    });
    assert!(!i.can_confirm());
    i.toggle(obj(1));
    assert!(!i.can_confirm());
    i.toggle(obj(2));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1), obj(2)]
        })
    );
}

#[test]
fn an_up_to_choice_can_be_confirmed_with_nothing_selected() {
    let i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] })
    );
}

#[test]
fn x_is_clamped_to_the_range_the_engine_offered() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 50,
    });
    assert_eq!(i.set_number(7), 7);
    // The client cannot express a value outside the offered range, so the
    // usual overflow tricks are simply unavailable to a player.
    assert_eq!(i.set_number(u32::MAX), 50);
    assert_eq!(i.set_number(4_000_000_000), 50);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(50)));
}

#[test]
fn x_starts_at_the_minimum() {
    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 3,
        max: 9,
    });
    assert_eq!(i.number(), 3);
}

#[test]
fn ordering_requires_every_offered_object_exactly_once() {
    let mut i = interaction(Pending::OrderObjects {
        player: me(),
        objects: vec![obj(1), obj(2), obj(3)],
    });
    i.toggle(obj(2));
    i.toggle(obj(3));
    assert!(!i.can_confirm(), "an incomplete order is not submittable");
    i.toggle(obj(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::OrderObjects {
            objects: vec![obj(2), obj(3), obj(1)]
        })
    );
}

#[test]
fn ordering_rejects_objects_that_were_not_offered() {
    let mut i = interaction(Pending::OrderObjects {
        player: me(),
        objects: vec![obj(1)],
    });
    assert_eq!(i.toggle(obj(42)), SelectionOutcome::Rejected);
}

#[test]
fn a_colour_choice_only_accepts_offered_colours() {
    let mut i = interaction(Pending::ChooseColor {
        player: me(),
        options: vec![ManaColor::White, ManaColor::Blue],
    });
    assert!(!i.can_confirm());
    assert!(!i.choose_index(2), "index beyond the offered options");
    assert!(i.choose_index(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseColor(ManaColor::Blue))
    );
}

/// The one pending choice the client could not answer at all. It is
/// worth a test of its own rather than a line in the colour one: the
/// mode was `Idle`, so every accessor said "nothing to do here" and the
/// game simply stopped.
#[test]
fn a_creature_type_choice_is_answerable() {
    let types: Vec<SubtypeId> = (0..350).map(SubtypeId::new).collect();
    let mut i = interaction(Pending::ChooseSubtype {
        player: me(),
        options: types.clone(),
    });
    assert!(!i.can_confirm(), "nothing is picked yet");
    assert_eq!(i.confirm(), None);
    assert!(!i.choose_index(350), "index beyond the offered types");
    assert!(i.choose_index(11));
    assert_eq!(i.chosen_index(), Some(11));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseSubtype(types[11])),
        "the answer names the type at the picked position"
    );
}

/// Every choice answered by position answers the same way, which is what
/// lets one chooser row in the renderer serve all four.
#[test]
fn every_indexed_choice_reports_the_row_it_picked() {
    let cases = [
        Pending::ChooseColor {
            player: me(),
            options: vec![ManaColor::Blue, ManaColor::Black],
        },
        Pending::ChoosePlayer {
            player: me(),
            options: vec![PlayerId::new(0), PlayerId::new(1)],
        },
        Pending::ChooseSubtype {
            player: me(),
            options: vec![SubtypeId::new(0), SubtypeId::new(1)],
        },
    ];
    for pending in cases {
        let mut i = interaction(pending);
        assert_eq!(i.chosen_index(), None, "nothing is picked to begin with");
        assert!(i.choose_index(1));
        assert_eq!(i.chosen_index(), Some(1));
        assert!(i.confirm().is_some(), "a picked row is submittable");
    }
}

#[test]
fn a_player_choice_only_accepts_offered_seats() {
    let mut i = interaction(Pending::ChoosePlayer {
        player: me(),
        options: vec![PlayerId::new(2), PlayerId::new(3)],
    });
    assert!(!i.choose_index(5));
    assert!(i.choose_index(0));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChoosePlayer(PlayerId::new(2)))
    );
}

#[test]
fn declaring_attackers_checks_both_the_creature_and_the_defender() {
    let mut i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1)]));

    assert!(!i.declare_attacker(obj(9), seat(1)), "not a candidate");
    assert!(!i.declare_attacker(obj(1), seat(7)), "not a defender");
    assert!(i.declare_attacker(obj(1), seat(1)));

    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(1))]
        })
    );
}

#[test]
fn re_declaring_an_attacker_replaces_its_defender_rather_than_duplicating() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), seat(2)]));
    i.declare_attacker(obj(1), seat(1));
    i.declare_attacker(obj(1), seat(2));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(2))]
        })
    );
}

#[test]
fn declaring_no_attackers_is_a_valid_answer() {
    let i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn blockers_must_block_an_actual_attacker() {
    let mut i = interaction(Pending::ChooseBlockers {
        player: me(),
        attacker: PlayerId::new(1),
        blockers: vec![BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        }],
    });
    assert!(!i.declare_blocker(obj(10), obj(99)));
    assert!(i.declare_blocker(obj(10), obj(1)));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareBlockers {
            blockers: vec![(obj(10), obj(1))]
        })
    );
}

/// A blocking choice with one attacker per listed blocker.
fn block_choice(options: Vec<BlockOption>) -> Pending {
    Pending::ChooseBlockers {
        player: me(),
        attacker: PlayerId::new(1),
        blockers: options,
    }
}

// The bug this whole pairing model exists to close: tapping a creature in
// combat pushed it onto the generic selection list, which `confirm` never
// reads for these two modes. A player could light up their entire board
// and still declare no attackers — the client looked like it had combat
// and did not.
#[test]
fn tapping_a_creature_in_combat_actually_declares_it() {
    let mut i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1)]));
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert!(i.is_selected(obj(1)), "a declared attacker reads as chosen");
    assert_eq!(i.declared(), 1);
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(1))]
        })
    );
}

#[test]
fn tapping_a_declared_attacker_again_calls_it_off() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    i.toggle(obj(1));
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Removed);
    assert!(!i.is_selected(obj(1)));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn a_table_with_one_defender_needs_no_aiming_at_all() {
    // The two-player case has to cost nothing: one thing to attack, and
    // the focus already on it before the player touches anything.
    let i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert_eq!(i.combat_focus(), CombatFocus::Defender(seat(1)));
}

#[test]
fn attacks_go_where_the_focus_points_and_the_focus_can_be_moved() {
    let mut i = interaction(attack_choice(
        vec![obj(1), obj(2)],
        vec![seat(1), seat(2), Defender::Planeswalker(obj(50))],
    ));
    i.toggle(obj(1));
    assert_eq!(i.cycle_focus(1), Some(Pick::Seat(PlayerId::new(2))));
    i.toggle(obj(2));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(1)), (obj(2), seat(2))]
        }),
        "two attackers, two different seats"
    );
    // And it wraps in both directions, so one key is enough to reach
    // every defender at a four-player table.
    assert_eq!(i.cycle_focus(-1), Some(Pick::Seat(PlayerId::new(1))));
    assert_eq!(
        i.cycle_focus(-1),
        Some(Pick::Object(obj(50))),
        "stepping back past the start wraps round"
    );
}

#[test]
fn tapping_a_planeswalker_aims_at_it() {
    let walker = Defender::Planeswalker(obj(50));
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), walker]));
    // A pointer should never have to find a cycle key: the thing being
    // attacked is on the table and can be tapped.
    assert_eq!(i.toggle(obj(50)), SelectionOutcome::Added);
    assert_eq!(i.combat_focus(), CombatFocus::Defender(walker));
    i.toggle(obj(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), walker)]
        })
    );
    assert_eq!(i.assignment(obj(1)), Some(CombatFocus::Defender(walker)));
}

#[test]
fn blocks_are_paired_with_the_attacker_in_focus() {
    let mut i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1), obj(2)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    i.toggle(obj(10));
    // Tap the second attacker to aim at it, then the blocker for it.
    assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(2)));
    i.toggle(obj(11));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareBlockers {
            blockers: vec![(obj(10), obj(1)), (obj(11), obj(2))]
        })
    );
}

#[test]
fn a_block_the_rules_forbid_is_refused_rather_than_sent() {
    // Evasion is a pairing question — a flier is a legal blocker and
    // still not a legal block — so the client must not send it and wait
    // for the engine to bounce it.
    let mut i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    assert_eq!(
        i.toggle(obj(11)),
        SelectionOutcome::Rejected,
        "obj(11) may only block obj(2)"
    );
    assert_eq!(i.declared(), 0);
    // The same creature against the attacker it *can* block goes through.
    i.cycle_focus(1);
    assert_eq!(i.toggle(obj(11)), SelectionOutcome::Added);
}

#[test]
fn calling_off_combat_forgets_the_declarations_and_the_aim() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), seat(2)]));
    i.cycle_focus(1);
    i.toggle(obj(1));
    i.cancel();
    assert_eq!(i.declared(), 0);
    assert_eq!(i.combat_focus(), CombatFocus::Defender(seat(1)));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn a_creature_that_cannot_attack_is_refused() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert_eq!(i.toggle(obj(99)), SelectionOutcome::Rejected);
    assert_eq!(i.declared(), 0);
}

#[test]
fn priority_confirms_as_a_pass_and_exposes_the_legal_actions() {
    let legal = LegalActions {
        can_pass: true,
        lands: vec![obj(1)],
        castable: vec![obj(2)],
        mana_abilities: vec![obj(3)],
        abilities: vec![(obj(4), 1)],
        suspendable: vec![],
    };
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(legal),
    });
    assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
    assert!(i.legal_actions().is_some());
}

/// A priority window on somebody else's turn does not say it is yours.
///
/// The same `Pending`, the same buttons under it, two sentences — and
/// the German is where it was worst: "Du bist dran" over Pass and Skip
/// says *it is your turn* in a way "Your move" only implies. Both
/// languages are asserted because a phrase that reads right in one and
/// wrong in the other is exactly what `Phrase` exists to stop.
#[test]
fn the_bar_says_whose_turn_it_is_over_the_same_two_buttons() {
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
        }),
    });
    assert_eq!(i.prompt().headline(Lang::En, Turn::Mine, None), "Your move");
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None),
        "Du bist dran"
    );
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Theirs, None),
        "You may respond"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Theirs, None),
        "Du kannst reagieren"
    );
}

/// And `Turn` is read off the seat, not guessed at.
#[test]
fn a_turn_belongs_to_the_seat_that_is_active() {
    assert_eq!(Turn::of(me(), me()), Turn::Mine);
    assert_eq!(Turn::of(PlayerId::new(1), me()), Turn::Theirs);
}

#[test]
fn playing_a_card_maps_to_the_right_action_and_refuses_illegal_ones() {
    let legal = LegalActions {
        can_pass: true,
        lands: vec![obj(1)],
        castable: vec![obj(2)],
        mana_abilities: vec![],
        abilities: vec![],
        suspendable: vec![],
    };
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(legal),
    });
    assert_eq!(
        i.play_card(obj(1)),
        Some(PlayerAction::PlayLand { card: obj(1) })
    );
    assert_eq!(
        i.play_card(obj(2)),
        Some(PlayerAction::CastSpell { card: obj(2) })
    );
    // A card the engine did not list is not playable, whatever the board
    // looks like.
    assert_eq!(i.play_card(obj(3)), None);
}

/// The bug that made a fetchland tap for mana instead of searching.
///
/// Flooded Strand is authored correctly — one `activated!` at index 0 with
/// `SearchLibrary`, and no mana ability on it. Chromatic Lantern grants
/// every land a mana ability, which puts the Strand in `mana_abilities`,
/// and the old guard fired on the index's numeric value: index 0 plus a
/// name in `mana_abilities` meant "mana ability", whatever the engine had
/// actually offered at that index.
///
/// Nothing in `abilities.rs` covered this shape, because nothing there put
/// index 0 and a populated `mana_abilities` on the same object.
#[test]
fn an_offered_ability_at_index_zero_beats_a_granted_mana_ability() {
    let strand = obj(1);
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            // Both, which is the whole situation: the grant names the
            // land, and its own printed ability is offered at index 0.
            mana_abilities: vec![strand],
            abilities: vec![(strand, 0)],
            suspendable: vec![],
        }),
    });
    assert_eq!(
        i.activate(strand, 0),
        Some(PlayerAction::ActivateAbility {
            source: strand,
            ability_index: 0,
        }),
        "the fetchland tapped for mana instead of searching"
    );
}

/// …and the shortcut still works when it is the only thing offered.
#[test]
fn index_zero_is_the_mana_shortcut_when_nothing_else_was_offered_there() {
    let forest = obj(1);
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![forest],
            abilities: vec![],
            suspendable: vec![],
        }),
    });
    assert_eq!(
        i.activate(forest, 0),
        Some(PlayerAction::ActivateManaAbility { source: forest })
    );
}

#[test]
fn a_card_offered_as_both_a_land_and_a_spell_is_not_a_one_click_land() {
    let plains = obj(1);
    let mdfc = obj(2);
    let bolt = obj(3);
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![plains, mdfc],
            castable: vec![mdfc, bolt],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
        }),
    });
    assert!(i.plays_only_as_a_land(plains));
    // In both lists: `play_card` checks lands first, so a one-click here
    // would make the spell face unreachable by mouse.
    assert!(!i.plays_only_as_a_land(mdfc));
    assert!(!i.plays_only_as_a_land(bolt));
    // And a card the engine never listed is neither.
    assert!(!i.plays_only_as_a_land(obj(9)));
}

#[test]
fn activating_an_ability_requires_it_to_have_been_offered() {
    let legal = LegalActions {
        can_pass: true,
        lands: vec![],
        castable: vec![],
        mana_abilities: vec![obj(5)],
        abilities: vec![(obj(6), 2)],
        suspendable: vec![],
    };
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(legal),
    });
    assert_eq!(
        i.activate(obj(5), 0),
        Some(PlayerAction::ActivateManaAbility { source: obj(5) })
    );
    assert_eq!(
        i.activate(obj(6), 2),
        Some(PlayerAction::ActivateAbility {
            source: obj(6),
            ability_index: 2
        })
    );
    assert_eq!(i.activate(obj(6), 3), None, "wrong ability index");
    assert_eq!(i.activate(obj(7), 0), None, "not a listed source");
}

#[test]
fn mulligan_and_yes_no_answers_are_mode_gated() {
    let mull = interaction(Pending::Mulligan {
        player: me(),
        taken: 1,
        next_is_free: false,
    });
    assert_eq!(mull.answer_mulligan(true), Some(PlayerAction::MulliganKeep));
    assert_eq!(
        mull.answer_mulligan(false),
        Some(PlayerAction::MulliganTake)
    );
    // A mulligan is not a yes/no question, and answering it as one is not
    // possible.
    assert_eq!(mull.answer_yes_no(true), None);

    let yn = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::Generic,
        source: None,
    });
    assert_eq!(yn.answer_yes_no(true), Some(PlayerAction::YesNo(true)));
    assert_eq!(yn.answer_mulligan(true), None);
}

#[test]
fn discarding_operates_on_the_hand_which_the_engine_leaves_implicit() {
    let mut i = interaction(Pending::DiscardChoice {
        player: me(),
        count: 2,
    });
    // No enumerated options, so any card in hand is fair game.
    assert!(i.selectable().is_empty());
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Full);
    assert!(i.can_confirm());
}

#[test]
fn cancelling_clears_a_selection_and_any_declarations() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    i.declare_attacker(obj(1), seat(1));
    i.cancel();
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn a_face_is_a_target_like_any_other() {
    // "Any target" (CR 115.4) spans objects and players, so one prompt
    // has to be answerable with either — or with both, when it takes two.
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![PlayerId::new(0), PlayerId::new(1)],
        min: 2,
        max: 2,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert!(!i.can_confirm());
    assert_eq!(i.toggle_player(PlayerId::new(1)), SelectionOutcome::Added);
    assert_eq!(
        i.selected_players().collect::<Vec<_>>(),
        vec![PlayerId::new(1)]
    );
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseTargets {
            objects: vec![obj(1)],
            players: vec![PlayerId::new(1)],
        })
    );
}

#[test]
fn a_seat_the_spell_cannot_reach_is_refused() {
    // The tab is a camera control the rest of the time, so a rejection
    // here is what lets the click fall through to the camera.
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(
        i.toggle_player(PlayerId::new(1)),
        SelectionOutcome::Rejected
    );
    i.toggle(obj(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        })
    );
}

#[test]
fn prompt_headlines_are_written_for_a_player_not_a_developer() {
    let i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 0,
        max: 2,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None),
        "Choose up to 2 targets"
    );

    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 50,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None),
        "Choose a number (0–50)"
    );

    let i = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
        source: None,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None),
        "Pay 2 life? Otherwise it enters tapped"
    );
}

/// Two lines in this `match` are about **another chair**, and both had
/// the seat in hand and printed a number. A draw offer is the one that
/// matters: at a table of four, "a draw was offered" is not a question
/// anybody can answer.
#[test]
fn the_two_lines_about_another_seat_say_whose_seat_it_is() {
    let mut roster = crate::test_support::statics(0);
    roster.seats.push(baylee_view::SeatIdentity {
        player: PlayerId::new(1),
        display_name: "AceVik".to_string(),
        is_ai: false,
        away: false,
        team: None,
    });

    let waiting = interaction(Pending::ChooseTargets {
        player: PlayerId::new(1),
        options: vec![obj(1)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    })
    .prompt();
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, Some(&roster)),
        "Warte auf AceVik"
    );

    let offer = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::DrawOffer {
            proposer: PlayerId::new(1),
        },
        source: None,
    })
    .prompt();
    assert_eq!(
        offer.headline(Lang::En, Turn::Mine, Some(&roster)),
        "AceVik offers a draw. Accept?"
    );
    assert_eq!(
        offer.headline(Lang::De, Turn::Mine, Some(&roster)),
        "AceVik bietet ein Remis an. Annehmen?"
    );

    // A frame drawn before `GameStatic` arrives numbers the seat rather
    // than dropping it: the sentence is about a chair that exists either
    // way, and this is what both lines said before there was a roster to
    // ask. The seat the roster does not describe answers the same way.
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, None),
        "Warte auf Platz 1"
    );
    assert_eq!(
        offer.headline(Lang::En, Turn::Mine, None),
        "Seat 1 offers a draw. Accept?"
    );
}

/// The backlog's own check, as a test: four card choices, and each line
/// has to be placeable without knowing which card asked it. Before this,
/// all four read "Choose 1 card(s)".
#[test]
fn four_card_choices_read_as_four_different_decisions() {
    let line = |reason, min, max, lang| {
        interaction(Pending::ChooseCards {
            player: me(),
            options: vec![obj(1), obj(2), obj(3)],
            min,
            max,
            prompt: reason,
        })
        .prompt()
        .headline(lang, Turn::Mine, None)
    };

    assert_eq!(
        line(ChoicePrompt::SearchLibrary, 1, 1, Lang::En),
        "Choose 1 card from your library"
    );
    assert_eq!(
        line(ChoicePrompt::ScryBottom, 0, 2, Lang::De),
        "Wähle bis zu 2 Karten, die nach unten gehen"
    );
    assert_eq!(
        line(ChoicePrompt::PutBackOnTop, 1, 1, Lang::De),
        "Wähle 1 Karte, die oben auf deine Bibliothek kommt"
    );
    assert_eq!(
        line(ChoicePrompt::Wish, 1, 3, Lang::En),
        "Choose 1–3 cards from outside the game"
    );
    // Delve is answered a line earlier: it is part of a cost, not a
    // selection, and that arm must not be shadowed by the reason noun.
    assert_eq!(
        line(ChoicePrompt::Delve, 0, 4, Lang::En),
        "Exile cards from your graveyard to help pay — each pays for one"
    );

    // And the whole of AS's second half: one card is never "card(s)".
    for lang in Lang::ALL {
        let one = line(ChoicePrompt::Generic, 1, 1, lang);
        assert!(!one.contains("(s)") && !one.contains("(n)"), "{one}");
    }
    assert_eq!(line(ChoicePrompt::Generic, 1, 1, Lang::De), "Wähle 1 Karte");
    assert_eq!(
        line(ChoicePrompt::Generic, 2, 2, Lang::De),
        "Wähle 2 Karten"
    );
}

#[test]
fn every_pending_variant_produces_a_prompt_without_panicking() {
    // A completeness guard: adding a choice to the engine without teaching
    // the client about it should fail here rather than at the table.
    let all = vec![
        Pending::Mulligan {
            player: me(),
            taken: 0,
            next_is_free: true,
        },
        Pending::MulliganBottom {
            player: me(),
            count: 1,
        },
        Pending::Priority {
            player: me(),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        },
        attack_choice(vec![obj(1)], vec![seat(1)]),
        Pending::ChooseBlockers {
            player: me(),
            attacker: PlayerId::new(1),
            blockers: vec![],
        },
        Pending::DiscardChoice {
            player: me(),
            count: 1,
        },
        Pending::LegendChoice {
            player: me(),
            options: vec![obj(1), obj(2)],
        },
        Pending::ChooseCards {
            player: me(),
            options: vec![],
            min: 0,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        Pending::ChooseTargets {
            player: me(),
            options: vec![],
            player_options: vec![],
            min: 0,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        Pending::ChooseSubtype {
            player: me(),
            options: vec![],
        },
        Pending::ChooseColor {
            player: me(),
            options: vec![ManaColor::White],
        },
        Pending::ChooseNumber {
            player: me(),
            min: 0,
            max: 1,
        },
        Pending::ChoosePlayer {
            player: me(),
            options: vec![me()],
        },
        Pending::ChooseCastMode {
            player: me(),
            object: obj(1),
            options: vec![],
        },
        Pending::OrderObjects {
            player: me(),
            objects: vec![],
        },
        Pending::YesNo {
            player: me(),
            prompt: YesNoPrompt::Generic,
            source: None,
        },
    ];
    for pending in all {
        let i = interaction(pending);
        assert!(!i.prompt().headline(Lang::En, Turn::Mine, None).is_empty());
    }
}

/// Convoke and delve say what they are, in both languages.
///
/// Reported from a live game: a waterbend spell "wollte von mir 99
/// Targets". Both halves of that were true — the engine published the
/// question in the targeting variant with a sentinel bound — and both are
/// fixed at the source. What is pinned here is the half a player reads:
/// the line must not be the "choose N targets" one, because tapping your
/// own creatures to help pay is not choosing a target for anything.
#[test]
fn helping_to_pay_is_not_asked_for_as_targeting() {
    let convoke = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 0,
        max: 2,
        reason: TargetPrompt::Convoke,
    });
    let delve = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1)],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Delve,
    });
    let targeting = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 0,
        max: 2,
        reason: TargetPrompt::Targets,
    });
    for lang in [Lang::En, Lang::De] {
        let target_line = targeting.prompt().headline(lang, Turn::Mine, None);
        for asking in [&convoke, &delve] {
            let line = asking.prompt().headline(lang, Turn::Mine, None);
            assert!(!line.is_empty());
            assert_ne!(
                line, target_line,
                "helping to pay was asked for as targeting in {lang:?}"
            );
        }
    }
    // And the selection itself is unchanged: it is still a bounded pick
    // over the offered permanents, answerable with none.
    assert!(convoke.confirm().is_some(), "convoke may be declined");
}

/// A target prompt over objects and seats.
fn target_choice(
    options: Vec<ObjectId>,
    player_options: Vec<PlayerId>,
    min: u8,
    max: u8,
) -> Pending {
    Pending::ChooseTargets {
        player: me(),
        options,
        player_options,
        min,
        max,
        reason: TargetPrompt::Targets,
    }
}

// Forty Soldiers are one card on the table, and the question "which four
// of them" has no answer a player could mean differently: they are
// identical. So the stack takes clicks the way a card takes one.
#[test]
fn a_counted_stack_takes_one_click_per_member() {
    let members = [obj(1), obj(2), obj(3), obj(4)];
    let mut i = interaction(target_choice(members.to_vec(), vec![], 1, 2));
    assert_eq!(i.toggle_group(&members), SelectionOutcome::Added);
    assert_eq!(i.toggle_group(&members), SelectionOutcome::Added);
    assert_eq!(i.pick_count(), 2);
    assert_eq!(
        i.picks(),
        [Pick::Object(obj(1)), Pick::Object(obj(2))],
        "two clicks pick two different Soldiers, not the same one twice"
    );
    assert_eq!(
        i.toggle_group(&members),
        SelectionOutcome::Full,
        "a third pick past `max` is refused, not silently swapped in"
    );
    assert_eq!(i.pick_count(), 2);
}

#[test]
fn a_stack_with_nothing_left_to_pick_takes_the_last_one_back() {
    let members = [obj(1), obj(2)];
    let mut i = interaction(target_choice(members.to_vec(), vec![], 0, 4));
    i.toggle_group(&members);
    i.toggle_group(&members);
    assert_eq!(i.pick_count(), 2);
    // Every member is picked and `max` is not reached, so the click can
    // only mean "one fewer" — which on a stack of one is the toggle it
    // has always been.
    assert_eq!(i.toggle_group(&members), SelectionOutcome::Removed);
    assert_eq!(i.picks(), [Pick::Object(obj(1))]);
}

#[test]
fn the_aim_walks_the_offer_and_reaches_a_seat() {
    let mut i = interaction(target_choice(
        vec![obj(1), obj(2)],
        vec![PlayerId::new(1)],
        1,
        1,
    ));
    assert_eq!(i.aim(), Some(Pick::Object(obj(1))));
    assert_eq!(i.focus_position(), Some((0, 3)));
    assert_eq!(i.cycle_focus(1), Some(Pick::Object(obj(2))));
    assert_eq!(
        i.cycle_focus(1),
        Some(Pick::Seat(PlayerId::new(1))),
        "a face is a target like a permanent is (CR 115.4), so the aim \
         reaches it without a second key"
    );
    assert_eq!(
        i.cycle_focus(1),
        Some(Pick::Object(obj(1))),
        "and it wraps, so one key covers the whole offer"
    );
}

#[test]
fn a_pick_is_taken_back_one_at_a_time() {
    let mut i = interaction(target_choice(
        vec![obj(1), obj(2)],
        vec![PlayerId::new(1)],
        0,
        3,
    ));
    i.toggle(obj(1));
    i.toggle_player(PlayerId::new(1));
    i.toggle(obj(2));
    assert_eq!(i.take_back(), Some(Pick::Object(obj(2))));
    assert_eq!(
        i.take_back(),
        Some(Pick::Seat(PlayerId::new(1))),
        "objects and seats are one order, or `the last pick` means nothing"
    );
    assert_eq!(i.picks(), [Pick::Object(obj(1))]);
    assert_eq!(i.take_back(), Some(Pick::Object(obj(1))));
    assert_eq!(i.take_back(), None, "and it stops at empty");
}

#[test]
fn a_creature_that_cannot_block_the_aimed_attacker_is_not_offered() {
    // The one place aiming changes what is *offered*: `BlockOption` is
    // per blocker, so a flier in the focus leaves the ground with nothing
    // to answer. Lighting it anyway invites a click `toggle` then refuses.
    let mut i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    assert!(i.is_selectable(obj(10)));
    assert!(
        !i.is_selectable(obj(11)),
        "obj(11) may only block obj(2), which is not what is aimed at"
    );
    assert!(i.is_selectable(obj(2)), "an attacker is always aimable");
    i.cycle_focus(1);
    assert!(i.is_selectable(obj(11)));
    assert!(!i.is_selectable(obj(10)));
}

/// Cancel is an *empty* answer, and only a question that will take one
/// has it to offer.
///
/// The zone browser's footer rests entirely on this: there is no cancel
/// action on the wire, so the way out of a "you may search" is to send
/// the empty set, and a question with a minimum above zero has no way out
/// at all. Both halves are asserted, and so is the order — [`cancel`]
/// first and [`confirm`] after, because a player who ticked a card and
/// then changed their mind must not have that card sent under the word
/// "Cancel".
///
/// [`cancel`]: Interaction::cancel
/// [`confirm`]: Interaction::confirm
#[test]
fn a_question_that_takes_nothing_is_answered_with_nothing() {
    let search = |min: u8| {
        Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: vec![obj(1), obj(2)],
                min,
                max: 2,
                prompt: ChoicePrompt::SearchLibrary,
            },
            me(),
        )
    };

    let may = search(0);
    assert_eq!(may.bounds(), Some((0, 2)));
    assert_eq!(
        may.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] }),
        "an empty answer to a may-search is not an answer at all"
    );

    let must = search(1);
    assert_eq!(must.bounds(), Some((1, 2)));
    assert!(
        must.confirm().is_none(),
        "a question with a minimum has a way out it cannot deliver"
    );

    // And the order: a pick taken back before the send.
    let mut mind_changed = search(0);
    mind_changed.toggle(obj(1));
    assert_eq!(
        mind_changed.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        })
    );
    mind_changed.cancel();
    assert_eq!(
        mind_changed.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] }),
        "Cancel sent the card the player had just decided against"
    );
}
