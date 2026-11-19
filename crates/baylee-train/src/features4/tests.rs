#[cfg(test)]
#[path = "tactical_tests.rs"]
mod tactical_tests;

#[cfg(test)]
#[path = "foretell_tests.rs"]
mod foretell_tests;

#[cfg(test)]
#[path = "tower_supplies_tests.rs"]
mod tower_supplies_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_zero_is_air_elemental_and_is_distinct_from_missing_identity() {
        let def = baylee_cards::by_index(CardIndex::new(0)).unwrap();
        assert_eq!(def.name(), "Air Elemental");
        let expected = encode_card_face(def, 0).summary;
        assert!(expected.iter().any(|v| *v != 0.0));
        let state = EncodedState4 {
            entity_card_ids: vec![0, UNKNOWN_CARD],
            ..EncodedState4::default()
        };
        assert_eq!(state.entity_dsl_summary(0), expected);
        assert_eq!(state.entity_dsl_summary(1), [0.0; CARD_DSL_WIDTH]);
        let mut candidate = Candidate4::default();
        assert_eq!(candidate.dsl_summary(&state), [0.0; CARD_DSL_WIDTH]);
        candidate.card_id = 0;
        assert_eq!(candidate.dsl_summary(&state), expected);
        let table = crate::carddsl::card_summary_table(&[UNKNOWN_CARD, 0]);
        assert_eq!(&table[..CARD_DSL_WIDTH], &[0.0; CARD_DSL_WIDTH]);
        assert_eq!(&table[CARD_DSL_WIDTH..], &expected);
    }
    use crate::housedeck::HouseDeck;
    use crate::selfplay::table_for;
    use baylee_core::preset::AIProfile;
    use baylee_gamehost::Session;

    #[test]
    fn naming_candidates_cover_the_public_registry_with_structural_features() {
        let d = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d, &d], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let view = session.observe_view(PlayerId::new(0));
        let pending = Pending::ChooseCardName {
            player: PlayerId::new(0),
            types: TypeSet::CREATURE,
            excluded_types: TypeSet::EMPTY,
        };
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let candidates = encode_candidates4(&state, &view, &pending);
        let expected: usize = baylee_cards::all()
            .map(|def| {
                def.faces
                    .iter()
                    .filter(|face| face.types.intersects(TypeSet::CREATURE))
                    .count()
            })
            .sum();
        assert_eq!(candidates.candidates.len(), expected);
        assert!(expected > 64);
        for candidate in candidates.candidates {
            let Some(PlayerAction::ChooseCardName { card, face }) = candidate.action else {
                panic!("name action")
            };
            assert_eq!(
                candidate.dsl_summary(&state),
                encode_card_face(baylee_cards::by_index(card).unwrap(), face as usize).summary
            );
        }
    }

    /// "Choose a nonland card name" (Cabal Therapy): no land face is offered.
    /// The engine refuses one, so a candidate for it is an answer the net could
    /// pick and be refused for.
    #[test]
    fn naming_candidates_leave_out_the_excluded_types() {
        let d = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d, &d], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let view = session.observe_view(PlayerId::new(0));
        let pending = Pending::ChooseCardName {
            player: PlayerId::new(0),
            types: TypeSet::EMPTY,
            excluded_types: TypeSet::LAND,
        };
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let candidates = encode_candidates4(&state, &view, &pending);
        let expected: usize = baylee_cards::all()
            .map(|def| {
                def.faces
                    .iter()
                    .filter(|face| !face.types.intersects(TypeSet::LAND))
                    .count()
            })
            .sum();
        assert_eq!(candidates.candidates.len(), expected);
        for candidate in candidates.candidates {
            let Some(PlayerAction::ChooseCardName { card, face }) = candidate.action else {
                panic!("name action")
            };
            let printed = &baylee_cards::by_index(card).unwrap().faces[usize::from(face)];
            assert!(!printed.types.intersects(TypeSet::LAND), "{}", printed.name);
        }
    }

    #[test]
    fn observation_digest_ignores_transport_but_tracks_priority_and_phase() {
        let d = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d, &d], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let view = session.observe_view(PlayerId::new(0));
        let mut changed = view.clone();
        changed.seq += 1;
        assert_eq!(
            board_observation_hash(&view),
            board_observation_hash(&changed)
        );
        changed.phase = Phase::SecondMain;
        assert_ne!(
            board_observation_hash(&view),
            board_observation_hash(&changed)
        );
        changed = view.clone();
        changed.awaiting = Some(PlayerId::new(1));
        assert_ne!(
            board_observation_hash(&view),
            board_observation_hash(&changed)
        );
    }

    #[test]
    fn eight_seat_relative_coordinates_round_trip_and_never_alias() {
        for seat_count in 2..=8 {
            for viewer_idx in 0..seat_count {
                let viewer = PlayerId::new(viewer_idx as u8);
                for s_idx in 0..seat_count {
                    let s = PlayerId::new(s_idx as u8);
                    let r = rel_seat(viewer, Some(s), seat_count);
                    assert!(r < seat_count);
                    if s == viewer {
                        assert_eq!(r, 0);
                    } else {
                        assert_ne!(r, 0, "opponent seat must never alias to 0");
                    }
                    assert_eq!(abs_seat(viewer, r, seat_count), s);
                }
                assert_eq!(rel_seat(viewer, None, seat_count), MAX_SEATS);
            }
        }
    }

    #[test]
    fn multiplayer_side_utility_sums_consistently() {
        // 4-player FFA: one winner gets 1.0, others 0.0
        let ffa_teams = [None, None, None, None];
        assert_eq!(side_utility(2, &ffa_teams, &[0, 1, 2, 3], &[2]), 1.0);
        assert_eq!(side_utility(0, &ffa_teams, &[0, 1, 2, 3], &[2]), 0.0);
        // 4-player FFA draw with 4 surviving seats: each gets 0.25
        for s in 0..4 {
            assert!((side_utility(s, &ffa_teams, &[0, 1, 2, 3], &[]) - 0.25).abs() < 1e-6);
        }
        // 2v2 team game draw with both sides alive: each surviving side gets 0.5
        let team_2v2 = [Some(1), Some(1), Some(2), Some(2)];
        assert!((side_utility(0, &team_2v2, &[0, 2], &[]) - 0.5).abs() < 1e-6);
        // 2v2 team win where seat 0 survived and teammate seat 1 was eliminated:
        // surviving winner gets 1.0, eliminated teammate on winning team gets 0.40,
        // losing team gets 0.0.
        assert!((side_utility(0, &team_2v2, &[0], &[0, 1]) - 1.0).abs() < 1e-6);
        assert!((side_utility(1, &team_2v2, &[0], &[0, 1]) - 0.40).abs() < 1e-6);
        assert!((side_utility(2, &team_2v2, &[0], &[0, 1]) - 0.0).abs() < 1e-6);
        assert!((side_utility(3, &team_2v2, &[0], &[0, 1]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn choose_mode_options_preserve_distinct_cast_mode_kinds_and_costs() {
        use baylee_engine::choice::CastModeDesc;
        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let pending = Pending::ChooseCastMode {
            player: PlayerId::new(0),
            object: ObjectId::new(1, 0),
            options: vec![
                CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: ManaCost::try_parse("{1}{U}").unwrap(),
                },
                CastModeDesc {
                    index: 1,
                    kind: CastModeKind::Kicked,
                    cost: ManaCost::try_parse("{3}{U}{U}").unwrap(),
                },
                CastModeDesc {
                    index: 2,
                    kind: CastModeKind::Flashback,
                    cost: ManaCost::try_parse("{4}{U}").unwrap(),
                },
            ],
        };
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        assert_eq!(cands.candidates.len(), 3);
        // Verify that the three options have distinct CastModeKind and ManaCost vectors
        assert_ne!(
            cands.candidates[0].feat[16..46],
            cands.candidates[1].feat[16..46]
        );
        assert_ne!(
            cands.candidates[1].feat[16..46],
            cands.candidates[2].feat[16..46]
        );
    }

    /// Madness is an alternative cost (CR 702.35b): it marks slot 28, so the
    /// candidate width the trained nets read stays the same.
    #[test]
    fn a_madness_cost_marks_the_alternative_cost_slot() {
        use baylee_engine::choice::CastModeDesc;
        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let option = |index, kind| CastModeDesc {
            index,
            kind,
            cost: ManaCost::try_parse("{1}{U}").unwrap(),
        };
        let pending = Pending::ChooseCastMode {
            player: PlayerId::new(0),
            object: ObjectId::new(1, 0),
            options: vec![
                option(0, CastModeKind::Madness),
                option(1, CastModeKind::Alternative(0)),
            ],
        };
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        assert_eq!(cands.candidates[0].feat[28], 1.0);
        assert_eq!(
            cands.candidates[0].feat[26..46],
            cands.candidates[1].feat[26..46]
        );
    }

    #[test]
    fn encode_teacher_action4_returns_none_when_action_unrepresented() {
        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();
        let pending = Pending::ChooseCardName {
            player: PlayerId::new(0),
            types: TypeSet::EMPTY,
            excluded_types: TypeSet::EMPTY,
        };
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        let unrepresented = PlayerAction::ActivateAbility {
            source: ObjectId::new(99999, 0),
            ability_index: 99,
        };
        assert!(
            encode_teacher_action4(&cands, &pending, &unrepresented).is_none(),
            "unrepresented teacher choice must return None rather than silently falling back to index 0"
        );
    }

    #[test]
    fn constrained_subset_sampler_normalizes_to_one_and_never_repairs() {
        use crate::deckgen::Rng;

        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();

        // 1. SubsetPick with count=2 out of 4 hand cards (DiscardChoice)
        let mut view_sub = view.clone();
        view_sub.hand.truncate(4);
        let pending_subset = Pending::DiscardChoice {
            player: PlayerId::new(0),
            count: 2,
        };
        let state = encode_state4(&view_sub, Some(&pending_subset), None, &[]);
        let cands = encode_candidates4(&state, &view_sub, &pending_subset);
        let logits = [1.2_f32, -0.4, 0.7, -1.1];
        let tau = 0.8_f32;

        let mut rng = Rng::new(12345);
        let mut unique_probs: BTreeMap<Vec<u8>, f64> = BTreeMap::new();
        for _ in 0..4096 {
            let (mask, free_mask, seq, lp) =
                sample_subset_constrained(&logits, &cands, &pending_subset, tau, &mut rng);
            let k: u8 = mask.iter().sum();
            assert_eq!(k, 2, "sampled subset size {k} must equal 2");
            let target = TeacherTarget4 {
                single_idx: 0,
                multi_mask: mask.clone(),
                free_mask: free_mask.clone(),
                pick_sequence: seq.clone(),
                scalar_norm: 0.0,
            };
            let ans = decode_action4(&cands, &view_sub, &pending_subset, &target)
                .expect("decodes subset cleanly");
            let reencoded = encode_teacher_action4(&cands, &pending_subset, &ans)
                .expect("decoded subset must re-encode cleanly");
            assert_eq!(
                reencoded.multi_mask, mask,
                "decode_action4 must never modify sampled mask"
            );
            assert_eq!(
                compute_subset_free_mask(&cands, &pending_subset, &mask, &seq),
                free_mask
            );
            assert_eq!(
                subset_free_mask_of(&cands, &pending_subset, &mask).as_ref(),
                Some(&free_mask),
                "the forced subset bits are the sampler's"
            );
            unique_probs.insert(mask, f64::from(lp).exp());
        }
        // A mask the constraints never draw has no free bits to read.
        assert_eq!(
            subset_free_mask_of(&cands, &pending_subset, &[1, 1, 1, 0]),
            None
        );
        assert_eq!(
            subset_free_mask_of(&cands, &pending_subset, &[1, 0, 0, 0]),
            None
        );
        // There are C(4,2) = 6 valid subsets of size 2
        assert_eq!(unique_probs.len(), 6);
        let total_prob: f64 = unique_probs.values().sum();
        assert!(
            (total_prob - 1.0).abs() < 1e-5,
            "constrained subset distribution must sum to 1.0, got {total_prob}"
        );
    }

    #[test]
    fn constrained_pair_sampler_normalizes_to_one_and_never_repairs() {
        use crate::deckgen::Rng;
        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();
        let tau = 0.8_f32;
        let mut rng = Rng::new(12345);

        // PairPick with 2 blockers and 2 attackers.
        let b0 = ObjectId::new(20, 0);
        let b1 = ObjectId::new(21, 0);
        let a0 = ObjectId::new(30, 0);
        let a1 = ObjectId::new(31, 0);
        let pair_cands = EncodedCandidates4 {
            head: HeadKind4::PairPick,
            pending_kind: PendingKind4::DeclareBlockers,
            min_choices: 0,
            max_choices: 2,
            candidates: Vec::new(),
            pairs: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
            pair_objects: vec![(b0, a0), (b0, a1), (b1, a0), (b1, a1)],
        };
        let pending_blk = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: Vec::new(),
            capacity: Vec::new(),
            obeying: Vec::new(),
            demands: Vec::new(),
            bounds: Vec::new(),
        };
        let pair_logits = [0.9_f32, -0.2, 0.3, -0.8];
        let mut pair_probs: BTreeMap<Vec<u8>, f64> = BTreeMap::new();
        for _ in 0..4096 {
            let (mask, free_mask, seq, lp) =
                sample_pairs_constrained(&pair_logits, &pair_cands, &pending_blk, tau, &mut rng);
            assert!(mask[0] + mask[1] <= 1);
            assert!(mask[2] + mask[3] <= 1);
            let target = TeacherTarget4 {
                single_idx: 0,
                multi_mask: mask.clone(),
                free_mask: free_mask.clone(),
                pick_sequence: seq.clone(),
                scalar_norm: 0.0,
            };
            let ans = decode_action4(&pair_cands, &view, &pending_blk, &target)
                .expect("decodes blockers cleanly");
            let PlayerAction::DeclareBlockers { blockers } = ans else {
                panic!("expected DeclareBlockers");
            };
            let mut rebuilt = vec![0_u8; 4];
            for (idx, po) in pair_cands.pair_objects.iter().enumerate() {
                if blockers.contains(po) {
                    rebuilt[idx] = 1;
                }
            }
            assert_eq!(
                rebuilt, mask,
                "decode_action4 must never repair sampled pair mask"
            );
            assert_eq!(compute_pair_free_mask(&pair_cands, &mask, &seq), free_mask);
            let lp = lp.expect("no attacker states a count, so every draw stands");
            assert_eq!(
                pair_free_mask_of(&pair_cands, &pending_blk, &mask).as_ref(),
                Some(&free_mask),
                "the forced pair bits are the sampler's"
            );
            pair_probs.insert(mask, f64::from(lp).exp());
        }
        assert_eq!(
            pair_free_mask_of(&pair_cands, &pending_blk, &[1, 1, 0, 0]),
            None,
            "one blocker on two attackers is never drawn"
        );
        // Valid assignments: 3 choices for b0 (none, a0, a1) * 3 choices for b1 (none, a0, a1) = 9
        assert_eq!(pair_probs.len(), 9);
        let total_pair_prob: f64 = pair_probs.values().sum();
        assert!(
            (total_pair_prob - 1.0).abs() < 1e-5,
            "constrained pair distribution must sum to 1.0, got {total_pair_prob}"
        );
    }

    /// An attacker that states it is blocked by two or more (menace) is never
    /// left with one blocker: a free creature that may block it joins, and
    /// where none is free it is left unblocked. The engine refused the lone
    /// blocker (`TooFewBlockers`), which the tactician's tests met on a table.
    #[test]
    fn the_pair_sampler_meets_the_counts_each_attacker_states() {
        use crate::deckgen::Rng;
        use baylee_engine::choice::AttackerBound;
        let b0 = ObjectId::new(20, 0);
        let b1 = ObjectId::new(21, 0);
        let a0 = ObjectId::new(30, 0);
        let a1 = ObjectId::new(31, 0);
        let cands = EncodedCandidates4 {
            head: HeadKind4::PairPick,
            pending_kind: PendingKind4::DeclareBlockers,
            min_choices: 0,
            max_choices: 2,
            candidates: Vec::new(),
            pairs: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
            pair_objects: vec![(b0, a0), (b0, a1), (b1, a0), (b1, a1)],
        };
        let menace = |min_blockers| Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: Vec::new(),
            capacity: Vec::new(),
            obeying: Vec::new(),
            demands: Vec::new(),
            bounds: vec![AttackerBound {
                attacker: a0,
                min_blockers,
                max_blockers: u32::MAX,
            }],
        };
        let sample = |logits: &[f32], pending: &Pending| {
            sample_pairs_constrained(logits, &cands, pending, 0.0, &mut Rng::new(3)).0
        };
        // b0 on a0, b1 nowhere: b1 joins b0 on a0.
        assert_eq!(sample(&[2.0, -1.0, -1.0, -3.0], &menace(2)), [1, 0, 1, 0]);
        // b0 on a0, b1 on a1: no one is free, so a0 goes unblocked.
        assert_eq!(sample(&[2.0, -1.0, -1.0, 3.0], &menace(2)), [0, 0, 0, 1]);
        // Three needed of two creatures: a0 goes unblocked.
        assert_eq!(sample(&[2.0, -1.0, 1.5, -3.0], &menace(3)), [0, 0, 0, 0]);
        // No stated count: the sample stands.
        let free = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: Vec::new(),
            capacity: Vec::new(),
            obeying: Vec::new(),
            demands: Vec::new(),
            bounds: Vec::new(),
        };
        assert_eq!(sample(&[2.0, -1.0, -1.0, -3.0], &free), [1, 0, 0, 0]);

        // A draw the counts changed is not the draw its free bits describe:
        // it has no log-probability. The same answer drawn as it stands keeps
        // its own (tau 0.8, so every pair is a real draw).
        let draw = |logits: &[f32]| {
            let (mask, _, _, lp) =
                sample_pairs_constrained(logits, &cands, &menace(2), 0.8, &mut Rng::new(3));
            (mask, lp)
        };
        let (mask, lp) = draw(&[20.0, -20.0, -20.0, -20.0]);
        assert_eq!((mask, lp), (vec![1, 0, 1, 0], None));
        let (mask, lp) = draw(&[20.0, -20.0, 20.0, -20.0]);
        assert_eq!(mask, [1, 0, 1, 0]);
        assert!(lp.is_some_and(|lp| lp.abs() < 1e-6), "{lp:?}");
    }

    /// The draw that stands as a blocker answer misses no stated count, and
    /// whether the counts can move a draw at all is read from the question:
    /// an attacker that needs two or more and that a creature may block, or
    /// one that takes fewer than may block it.
    #[test]
    fn the_counts_say_which_blocker_answers_stand_and_whether_any_moves() {
        use baylee_engine::choice::AttackerBound;
        let b0 = ObjectId::new(20, 0);
        let b1 = ObjectId::new(21, 0);
        let a0 = ObjectId::new(30, 0);
        let a1 = ObjectId::new(31, 0);
        let cands = EncodedCandidates4 {
            head: HeadKind4::PairPick,
            pending_kind: PendingKind4::DeclareBlockers,
            min_choices: 0,
            max_choices: 2,
            candidates: Vec::new(),
            pairs: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
            pair_objects: vec![(b0, a0), (b0, a1), (b1, a0), (b1, a1)],
        };
        let block = |bounds: Vec<AttackerBound>| Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: Vec::new(),
            capacity: Vec::new(),
            obeying: Vec::new(),
            demands: Vec::new(),
            bounds,
        };
        let bound = |attacker, min_blockers, max_blockers| AttackerBound {
            attacker,
            min_blockers,
            max_blockers,
        };
        let menace = block(vec![bound(a0, 2, u32::MAX)]);
        // A lone blocker on the menace attacker is moved, never drawn as it
        // stands; both on it, or none, stand.
        assert_eq!(pair_free_mask_of(&cands, &menace, &[1, 0, 0, 0]), None);
        assert_eq!(pair_free_mask_of(&cands, &menace, &[0, 0, 1, 0]), None);
        assert_eq!(
            pair_free_mask_of(&cands, &menace, &[1, 0, 1, 0]),
            Some(vec![1, 0, 1, 0])
        );
        assert_eq!(
            pair_free_mask_of(&cands, &menace, &[0, 1, 0, 1]),
            Some(vec![1, 1, 1, 1])
        );
        assert!(attacker_counts_may_move(&cands, &menace));
        for (bounds, moves) in [
            (Vec::new(), false),
            (vec![bound(a1, 1, u32::MAX)], false),
            (vec![bound(a1, 0, 2)], false),
            (vec![bound(a1, 0, 1)], true),
            (vec![bound(a0, 3, u32::MAX)], true),
            // Nothing may block a creature that is not attacking here.
            (vec![bound(ObjectId::new(32, 0), 2, u32::MAX)], false),
        ] {
            assert_eq!(
                attacker_counts_may_move(&cands, &block(bounds.clone())),
                moves,
                "{bounds:?}"
            );
        }
        let mulligan = Pending::Mulligan {
            player: PlayerId::new(0),
            taken: 0,
            next_is_free: false,
            can_take: true,
        };
        assert!(!attacker_counts_may_move(&cands, &mulligan));
    }

    #[test]
    fn multi_defender_choose_attackers_and_log_scaled_choose_number_round_trip() {
        for &(lo, hi) in &[(0_u32, 5_u32), (0, 20), (0, 1000), (1, 50)] {
            for v in [lo, u32::midpoint(lo, hi), hi] {
                let t = normalize_scalar_choice(v, lo, hi);
                assert!((-1.0 - 1e-5..=1.0 + 1e-5).contains(&t));
                let back = denormalize_scalar_choice(t, lo, hi);
                assert_eq!(
                    back, v,
                    "scalar choice {v} in [{lo}, {hi}] failed round-trip"
                );
            }
        }
    }
}
