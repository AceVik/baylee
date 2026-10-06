use super::*;

/// The three modes of Sheoldred's Edict, as the engine offers them.
///
/// No `Normal` option, because every effect the card has sits under a
/// mode and the wizard refuses a mode-less cast for such a card
/// (CR 700.2). That is what made the old answer — the position of
/// `Normal`, else nought — take the first printed mode at every table.
fn edict_modes() -> Vec<baylee_engine::choice::CastModeDesc> {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    (0..3)
        .map(|i| CastModeDesc {
            index: u8::try_from(i).unwrap(),
            kind: CastModeKind::Mode(i),
            cost: baylee_core::mana::ManaCost::ZERO,
        })
        .collect()
}

/// A view with Sheoldred's Edict on the stack and `theirs` opposite it.
fn edict_table(theirs: Vec<PublicObject>) -> PlayerView {
    let mut v = view(0, &[20, 20], theirs);
    v.stack = vec![carded(
        permanent(obj(9), PlayerId::new(0), 0),
        "Sheoldred's Edict",
        TypeSet::INSTANT,
    )];
    baylee_client_core::test_support::project_current_targets(&mut v);
    v
}

fn token_creature(id: ObjectId, controller: PlayerId) -> PublicObject {
    let mut o = permanent(id, controller, 2);
    o.token = Some(1);
    o
}

/// "Choose one —" is chosen by what the mode reaches, not by where it is
/// printed.
///
/// Sheoldred's Edict is the pool's clearest case: all three modes are
/// always offered, because none of them targets — an edict names no
/// target at all (CR 115.1) — so the engine's own "a mode that cannot
/// find its targets is not offered" filter says nothing about any of
/// them. Against a lone planeswalker the printed first mode asks for a
/// nontoken creature and does nothing whatsoever, and that is what this
/// agent used to choose every time.
///
/// The nontoken case is the control: it is the answer the old code gave
/// as well, so a test containing only it would pass against the defect.
/// It is a *carded* creature rather than the bare fixture, because
/// `IsToken` is only readable of an object that carries one of the two
/// handles — a bare permanent is the pair the view cannot tell apart, and
/// the control would then agree by falling back instead of by reading.
#[test]
fn a_modal_spell_takes_the_mode_that_reaches_something() {
    let them = PlayerId::new(1);
    let theirs = carded(
        permanent(obj(1), them, 2),
        "Baleful Strix",
        TypeSet::CREATURE,
    );
    for (what, board, expected) in [
        ("a lone planeswalker", vec![walker(obj(1), them, 4)], 2),
        ("a lone token", vec![token_creature(obj(1), them)], 1),
        ("a nontoken creature", vec![theirs.clone()], 0),
    ] {
        let v = edict_table(board);
        assert_eq!(
            agent().act(
                &v,
                &Pending::ChooseCastMode {
                    player: v.seat,
                    object: obj(9),
                    options: edict_modes(),
                }
            ),
            PlayerAction::ChooseMode(expected),
            "against {what}, only mode {expected} of Sheoldred's Edict does anything"
        );
    }
}

/// "Choose one or more —" is chosen by what each chosen mode reaches.
///
/// Farewell's fifteen sets, as the engine offers them: bit `i` is mode
/// `i`, in increasing order. A set holding a mode that exiles nothing is
/// paid for and partly idle, so it loses to every set without one; of
/// the rest, the one reaching the most wins; and "exile all graveyards",
/// which this agent cannot read, is not bought for its own sake, because
/// the earlier (smaller) set breaks the tie. Against a lone creature that
/// is the creatures alone. Against an artifact creature it is artifacts
/// and creatures both, which is the half the first answer cannot pass
/// by accident. Against nothing at all it is the graveyards: the one
/// mode that may do something beats the first printed, which does
/// nothing.
#[test]
fn a_spell_of_several_modes_takes_the_set_that_reaches_the_most() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let them = PlayerId::new(1);
    let options: Vec<CastModeDesc> = (1..16_u8)
        .map(|set| CastModeDesc {
            index: set - 1,
            kind: CastModeKind::Modes(set),
            cost: baylee_core::mana::ManaCost::parse("{4}{W}{W}"),
        })
        .collect();
    let strix = |types| vec![carded(permanent(obj(1), them, 2), "Baleful Strix", types)];
    for (what, board, expected) in [
        ("a creature", strix(TypeSet::CREATURE), 0b0010),
        (
            "an artifact creature",
            strix(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
            0b0011,
        ),
        ("nothing at all", vec![], 0b1000),
    ] {
        let mut v = view(0, &[20, 20], board);
        v.stack = vec![carded(
            permanent(obj(9), PlayerId::new(0), 0),
            "Farewell",
            TypeSet::SORCERY,
        )];
        baylee_client_core::test_support::project_current_targets(&mut v);
        let answer = agent().act(
            &v,
            &Pending::ChooseCastMode {
                player: v.seat,
                object: obj(9),
                options: options.clone(),
            },
        );
        let PlayerAction::ChooseMode(slot) = answer else {
            panic!("expected a set of modes, got {answer:?}")
        };
        assert_eq!(
            options[slot].kind,
            CastModeKind::Modes(expected),
            "against {what}"
        );
    }
}

/// "Choose a creature you control" reaches something when there is one
/// of the agent's own to choose. Final Showdown against a board with a
/// creature on each side: the destruction reaches, and so does keeping
/// the agent's own creature out of it, so both are bought. With nothing
/// of its own the choice would find nothing, and the destruction is
/// cast alone.
#[test]
fn a_spree_spell_buys_the_mode_that_saves_its_own_creature() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let options: Vec<CastModeDesc> = (1..8_u8)
        .map(|set| CastModeDesc {
            index: set - 1,
            kind: CastModeKind::Modes(set),
            cost: baylee_core::mana::ManaCost::ZERO,
        })
        .collect();
    let creature = |id, seat| {
        carded(
            permanent(obj(id), seat, 2),
            "Baleful Strix",
            TypeSet::CREATURE,
        )
    };
    for (what, board, expected) in [
        (
            "a creature each",
            vec![creature(1, me), creature(2, them)],
            0b110,
        ),
        ("only theirs", vec![creature(2, them)], 0b100),
    ] {
        let mut v = view(0, &[20, 20], board);
        v.stack = vec![carded(
            permanent(obj(9), me, 0),
            "Final Showdown",
            TypeSet::INSTANT,
        )];
        baylee_client_core::test_support::project_current_targets(&mut v);
        let answer = agent().act(
            &v,
            &Pending::ChooseCastMode {
                player: v.seat,
                object: obj(9),
                options: options.clone(),
            },
        );
        let PlayerAction::ChooseMode(slot) = answer else {
            panic!("expected a set of modes, got {answer:?}")
        };
        assert_eq!(
            options[slot].kind,
            CastModeKind::Modes(expected),
            "against {what}"
        );
    }
}

/// "Choose two" with nothing on the stack to counter: the engine offers
/// the three pairs of bounce, tap and draw. "Tap all creatures your
/// opponents control" is read off the board: against an opposing
/// creature it reaches and bounce plus tap is taken; against no creature
/// at all it would be paid for and idle, so bounce plus draw is. Before
/// the agent read the tap, both boards answered bounce plus tap.
#[test]
fn choose_two_leaves_out_a_tap_with_nothing_to_tap() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let options: Vec<CastModeDesc> = [0b0110_u8, 0b1010, 0b1100]
        .into_iter()
        .enumerate()
        .map(|(index, set)| CastModeDesc {
            index: u8::try_from(index).unwrap(),
            kind: CastModeKind::Modes(set),
            cost: baylee_core::mana::ManaCost::parse("{1}{U}{U}{U}"),
        })
        .collect();
    let land = |id, seat| carded(permanent(obj(id), seat, 0), "Island", TypeSet::LAND);
    let creature = carded(
        permanent(obj(2), them, 2),
        "Baleful Strix",
        TypeSet::CREATURE,
    );
    for (what, board, expected) in [
        (
            "a creature of theirs",
            vec![land(1, them), creature],
            0b0110,
        ),
        ("only lands", vec![land(1, them), land(3, me)], 0b1010),
    ] {
        let mut v = view(0, &[20, 20], board);
        v.stack = vec![carded(
            permanent(obj(9), me, 0),
            "Cryptic Command",
            TypeSet::INSTANT,
        )];
        baylee_client_core::test_support::project_current_targets(&mut v);
        let answer = agent().act(
            &v,
            &Pending::ChooseCastMode {
                player: v.seat,
                object: obj(9),
                options: options.clone(),
            },
        );
        let PlayerAction::ChooseMode(slot) = answer else {
            panic!("expected a set of modes, got {answer:?}")
        };
        assert_eq!(
            options[slot].kind,
            CastModeKind::Modes(expected),
            "against {what}"
        );
    }
}

/// An empty board is read, and the reading is that nothing is reached.
///
/// Every mode scores nought, so the printed order is all that is left and
/// the answer is the old one. Worth pinning because the tie-break is the
/// half that keeps this change invisible everywhere it has nothing to
/// say: `max_by_key` returns the *last* maximum, so without the
/// `Reverse(position)` in the key an empty table would answer 2.
#[test]
fn a_modal_spell_with_nothing_to_reach_keeps_its_printed_order() {
    let v = edict_table(vec![]);
    assert_eq!(
        agent().act(
            &v,
            &Pending::ChooseCastMode {
                player: v.seat,
                object: obj(9),
                options: edict_modes(),
            }
        ),
        PlayerAction::ChooseMode(0)
    );
}

/// A card that can also be cast normally still is.
///
/// The ranking decides between modes and never between a mode and the
/// printed cast: a mode is offered exactly when it is affordable, and
/// overload is the shape that makes "a mode" and "the expensive one" the
/// same thing. So a `Normal` option ends the question wherever there is
/// one.
///
/// The options are built by hand because no card in the pool offers this
/// pair today. All four `ModalSpell` cards here — Sheoldred's Edict,
/// Heliod's Intervention, Cyclonic Rift, Damn — put *every* effect under
/// a mode, so the wizard refuses them a `Normal` option (CR 700.2a) and
/// the two overload cards write their printed cast as `Mode(0)` beside
/// the overloaded `Mode(1)`. The guard exists for the card that does not,
/// and a fixture that cannot be printed yet is the only way to hold it
/// before one is.
#[test]
fn a_normal_cast_is_not_traded_for_a_mode() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let v = edict_table(vec![walker(obj(1), PlayerId::new(1), 4)]);
    let options = vec![
        CastModeDesc {
            index: 0,
            kind: CastModeKind::Mode(0),
            cost: baylee_core::mana::ManaCost::ZERO,
        },
        CastModeDesc {
            index: 1,
            kind: CastModeKind::Normal,
            cost: baylee_core::mana::ManaCost::ZERO,
        },
    ];
    assert_eq!(
        agent().act(
            &v,
            &Pending::ChooseCastMode {
                player: v.seat,
                object: obj(9),
                options,
            }
        ),
        PlayerAction::ChooseMode(1),
        "the printed cast is the answer even when a mode outscores it"
    );
}
