use super::*;

/// The position #123 was reported from: one attacker with first strike
/// and no evasion, `blockers` untapped creatures that may all legally
/// block it, and a life total it beats on its own.
///
/// `power` is what the view says about the attacker, so `None` is the
/// case where the view carries the attack but cannot describe what is
/// in it.
fn lethal_attack(
    power: Option<i16>,
    in_view: bool,
    life: i32,
    blockers: u32,
) -> (PlayerView, Pending) {
    let defender = PlayerId::new(0);
    let attacker = PlayerId::new(1);
    let mut battlefield: Vec<PublicObject> = (0..blockers)
        .map(|i| permanent(obj(10 + i), defender, 2))
        .collect();
    if in_view {
        let mut a = permanent(obj(1), attacker, power.unwrap_or(75));
        a.power = power;
        a.keywords = baylee_cards_dsl::KeywordSet::FIRST_STRIKE.bits();
        battlefield.push(a);
    }
    let mut v = view(0, &[life, 20], battlefield);
    v.active = attacker;
    v.step = baylee_view::Step::DeclareBlockers;
    v.combat.attackers = vec![baylee_view::AttackerView {
        creature: obj(1),
        defending: Defender::Player(defender),
        blocked: false,
    }];
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: defender,
        attacker,
        blockers: (0..blockers)
            .map(|i| baylee_engine::choice::BlockOption {
                blocker: obj(10 + i),
                attackers: vec![obj(1)],
            })
            .collect(),
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    (v, pending)
}

/// A menace attacker, and `blockers` untapped creatures the engine has
/// offered a pairing with. `attacker` is its printed size; a blocker is
/// an `n/n`.
fn menace_attack(attacker: (i16, i16), blockers: &[i16], life: i32) -> (PlayerView, Pending) {
    let defender = PlayerId::new(0);
    let attacking = PlayerId::new(1);
    let slot = |i: usize| obj(10 + u32::try_from(i).unwrap_or(0));
    let mut battlefield: Vec<PublicObject> = blockers
        .iter()
        .enumerate()
        .map(|(i, size)| permanent(slot(i), defender, *size))
        .collect();
    let mut a = permanent(obj(1), attacking, attacker.0);
    a.toughness = Some(attacker.1);
    a.keywords = baylee_cards_dsl::KeywordSet::MENACE.bits();
    battlefield.push(a);
    let mut v = view(0, &[life, 20], battlefield);
    v.active = attacking;
    v.step = baylee_view::Step::DeclareBlockers;
    v.combat.attackers = vec![baylee_view::AttackerView {
        creature: obj(1),
        defending: Defender::Player(defender),
        blocked: false,
    }];
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: defender,
        attacker: attacking,
        blockers: (0..blockers.len())
            .map(|i| baylee_engine::choice::BlockOption {
                blocker: slot(i),
                attackers: vec![obj(1)],
            })
            .collect(),
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    (v, pending)
}

fn blocks(profile: AIProfile, v: &PlayerView, pending: &Pending) -> usize {
    match HeuristicAgent::new(profile).act(v, pending) {
        PlayerAction::DeclareBlockers { blockers } => blockers.len(),
        other => panic!("not a block answer: {other:?}"),
    }
}

/// Our eight 2/2s against their one creature of `power` with
/// `keywords`, which is `tapped` when it attacked the turn before.
fn swing_back_board(
    lives: [i32; 2],
    power: i16,
    keywords: baylee_cards_dsl::KeywordSet,
    tapped: bool,
) -> (PlayerView, Pending) {
    let mut giant = keyworded(
        obj(20),
        PlayerId::new(1),
        power,
        "Macetail Hystrodon",
        keywords,
    );
    if tapped {
        giant.status = ObjectStatus::TAPPED;
    }
    let mut board: Vec<PublicObject> = (1..=8)
        .map(|i| permanent(obj(i), PlayerId::new(0), 2))
        .collect();
    board.push(giant);
    let v = view(0, &lives, board);
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: (1..=8).map(obj).collect(),
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    (v, pending)
}

/// Which of the eight a profile keeps home.
fn kept_home(profile: AIProfile, v: &PlayerView, pending: &Pending) -> Vec<ObjectId> {
    let PlayerAction::DeclareAttackers { attackers } = HeuristicAgent::new(profile).act(v, pending)
    else {
        panic!("not an attack answer")
    };
    (1..=8)
        .map(obj)
        .filter(|id| !attackers.iter().any(|(a, _)| a == id))
        .collect()
}

/// #123, the game the owner lost. A 75/75 first striker attacked, and on
/// the house AI's turn it is still tapped, so nothing can block and
/// every attack rule says swing: NOVICE, CASUAL, STEADY and SHARP sent
/// all eight for sixteen into twenty, and the next turn the 75/75 met a
/// table of tapped creatures and no block to offer. Only EXPERT, which
/// prices retaliation, kept one home.
///
/// With it untapped the three shallow profiles stay home anyway, since
/// it kills whatever attacks, but SHARP still sent all eight into it.
#[test]
fn a_table_that_would_die_to_the_swing_back_keeps_a_blocker_home() {
    use baylee_cards_dsl::KeywordSet;
    let (v, pending) = swing_back_board([20, 20], 75, KeywordSet::FIRST_STRIKE, true);
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            kept_home(profile, &v, &pending).len(),
            1,
            "{name} left nothing home to block a 75/75 that untaps next turn"
        );
    }
    let (v, pending) = swing_back_board([20, 20], 75, KeywordSet::FIRST_STRIKE, false);
    for (name, profile) in EVERY_PROFILE {
        assert!(
            !kept_home(profile, &v, &pending).is_empty(),
            "{name} sent all eight past an untapped 75/75"
        );
    }
}

/// The negatives that keep the pass from being "always keep one home".
/// A swing back of ten into twenty is survived with nothing home, so the
/// profiles that attacked with all eight still do (EXPERT's own pricing
/// of that retaliation is its search's business and is not pinned
/// here); and an attack that ends the game has nothing to survive, so
/// every profile sends all eight into sixteen life.
#[test]
fn a_swing_back_that_does_not_kill_keeps_nothing_home() {
    use baylee_cards_dsl::KeywordSet;
    let (v, pending) = swing_back_board([20, 20], 10, KeywordSet::FIRST_STRIKE, true);
    for (name, profile) in &EVERY_PROFILE[..4] {
        assert_eq!(
            kept_home(*profile, &v, &pending),
            vec![],
            "{name} held back against a swing back of ten at twenty life"
        );
    }
    let (v, pending) = swing_back_board([20, 16], 75, KeywordSet::FIRST_STRIKE, true);
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            kept_home(profile, &v, &pending),
            vec![],
            "{name} held back from an attack that ends the game"
        );
    }
}

/// What stays home is what can stop what comes back, read through the
/// same keywords the rest of combat reads. A vigilant attacker defends
/// from where it is, so nobody else has to stay. Against a flyer the one
/// creature with reach stays, not the cheapest; with no reach at all
/// nothing held back blocks it, so the table attacks as it meant to.
/// Menace takes two blockers. Trample pushes the excess through: one 2/2
/// in front of a 21-power trampler lets 19 into 20 life, while a 22-power
/// one needs a second.
#[test]
fn what_stays_home_is_what_can_block_what_comes_back() {
    use baylee_cards_dsl::KeywordSet;
    let (mut v, pending) = swing_back_board([20, 20], 75, KeywordSet::FIRST_STRIKE, true);
    v.battlefield[0].keywords = KeywordSet::VIGILANCE.bits();
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            kept_home(profile, &v, &pending),
            vec![],
            "{name}: the vigilant 2/2 blocks from the attack"
        );
    }
    let (mut v, pending) = swing_back_board([20, 20], 75, KeywordSet::FLYING, true);
    v.battlefield[7].keywords = KeywordSet::REACH.bits();
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            kept_home(profile, &v, &pending),
            vec![obj(8)],
            "{name}: only the reach creature can block a flyer"
        );
    }
    let (v, pending) = swing_back_board([20, 20], 75, KeywordSet::FLYING, true);
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            kept_home(profile, &v, &pending),
            vec![],
            "{name}: nothing home stops a flyer, so holding back buys nothing"
        );
    }
    // Phased out today is phased in on their untap step (CR 502.1). It
    // is tapped too, so that every attack rule says swing and the pass
    // is the only thing that can see it.
    let (mut v, pending) = swing_back_board([20, 20], 75, KeywordSet::FIRST_STRIKE, true);
    v.battlefield[8].status =
        ObjectStatus::from_bits(ObjectStatus::TAPPED.bits() | ObjectStatus::PHASED_OUT.bits());
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            kept_home(profile, &v, &pending).len(),
            1,
            "{name}: a phased-out 75/75 comes back all the same"
        );
    }
    // Double strike with trample: the second step meets no blocker
    // (CR 702.19d), so one 2/2 lets 22 - 2 = 20 through and two let 18.
    for (keywords, power, home) in [
        (KeywordSet::MENACE, 75, 2),
        (KeywordSet::TRAMPLE, 21, 1),
        (KeywordSet::TRAMPLE, 22, 2),
        (KeywordSet::TRAMPLE.union(KeywordSet::DOUBLE_STRIKE), 11, 2),
    ] {
        let (v, pending) = swing_back_board([20, 20], power, keywords, true);
        for (name, profile) in EVERY_PROFILE {
            assert_eq!(
                kept_home(profile, &v, &pending).len(),
                home,
                "{name} against a {power}-power {keywords:?}"
            );
        }
    }
}

/// #123, the scenario: a lethal attacker is chumped, first strike and
/// all. Eight blockers, one attacker — one of them is enough, and
/// spending a second on it would be the opposite error.
#[test]
fn a_lethal_attacker_is_chump_blocked_by_every_profile() {
    let (v, pending) = lethal_attack(Some(75), true, 20, 8);
    for (name, profile) in PROFILES {
        assert_eq!(blocks(profile, &v, &pending), 1, "{name} took the damage");
    }
}

/// #123, the negative that keeps the rule honest. The same board with
/// the attacker below lethal: blocking loses a 2/2 to kill nothing, so
/// every profile stays home. Without this, "always block" passes the
/// test above wearing rule 1's clothes.
#[test]
fn a_bad_trade_is_declined_while_the_seat_is_not_dying() {
    let (mut v, pending) = lethal_attack(Some(4), true, 20, 8);
    v.battlefield.last_mut().unwrap().toughness = Some(4);
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            0,
            "{name} chumped for nothing"
        );
    }
}

/// #123, first strike specifically: it is the one keyword the reported
/// attacker carried, and it changes [`combat::exchange`] without
/// changing legality (CR 702.7).
///
/// A 4/4 into a **4/4** is the exchange it decides, and the size is not
/// incidental: against a 2/2 the first striker kills and survives either
/// way, so the test would have moved the blocker's toughness and called
/// it a keyword. Here the keyword is the only thing that moves.
#[test]
fn first_strike_changes_the_exchange_and_not_the_legality() {
    let board = |first_strike: bool, life: i32| {
        let (mut v, pending) = lethal_attack(Some(4), true, life, 1);
        let attacker = v.battlefield.last_mut().unwrap();
        attacker.toughness = Some(4);
        if !first_strike {
            attacker.keywords = 0;
        }
        v.battlefield[0].power = Some(4);
        v.battlefield[0].toughness = Some(4);
        (v, pending)
    };
    // Ours dies and theirs walks away: not a trade, and the seat can
    // afford to decline it.
    let (v, pending) = board(true, 20);
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            0,
            "{name} fed a first striker"
        );
    }
    // The same board with the keyword gone and nothing else changed:
    // now both die, the trade is even, and it is taken.
    let (v, pending) = board(false, 20);
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            1,
            "{name} refused an even trade"
        );
    }
    // And first strike does not stop a chump block, because a creature
    // kept back is worth nothing after the game is over.
    let (v, pending) = board(true, 4);
    for (name, profile) in PROFILES {
        assert_eq!(blocks(profile, &v, &pending), 1, "{name} died to a 4/4");
    }
}

/// #123, the other half of the same sentence: the attack itself can be
/// missing from the view while the engine is offering pairings against
/// it. The creature reads perfectly — it is on the battlefield with a
/// power and a toughness — and `view.combat.attackers` does not name it,
/// so the damage sum was nought and the position read as safe.
///
/// `Pending::ChooseBlockers` names it, and that is the offer the seat is
/// being made, so it is the authority on what is attacking. The two
/// sources are unioned rather than one replacing the other: the view
/// carries the whole attack, including what this seat may not block.
#[test]
fn an_attack_the_view_does_not_carry_is_read_off_the_pairings() {
    let (mut v, pending) = lethal_attack(Some(75), true, 20, 8);
    v.combat.attackers = vec![];
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            1,
            "{name} read a 75/75 as nothing"
        );
    }
}

/// #123, the rule this ticket turned out to be about: an attacker the
/// view cannot describe is **unknown**, not absent.
///
/// `Fighter::of` is three `?` in a row — the object, its power, its
/// toughness — and each `None` used to leave the attacker out of the
/// damage sum *and* out of every blocker's candidate list. So the seat
/// read a lethal attack as no attack at all and declined every block:
/// the engine's own pairings said a creature was there, and the agent
/// answered as though the board were empty.
///
/// Both halves of the decision are pinned, because they fail
/// separately: `choose_blocks` reads the attack out of the view, and
/// `search::blockers` used to fall back to it on exactly this condition
/// — a fallback onto the same blind spot, which is not a fallback.
/// `NOVICE`/`CASUAL` take the first, the rest the second.
#[test]
fn an_attacker_the_view_cannot_describe_is_still_blocked() {
    // The control: the same position, readable. Without it the two
    // below would also pass against an agent that blocks with anything.
    let (v, pending) = lethal_attack(Some(75), true, 20, 8);
    for (name, profile) in PROFILES {
        assert_eq!(blocks(profile, &v, &pending), 1, "{name}, readable");
    }
    // The view carries the attack and the engine offers the pairings,
    // but the attacker has no body on it.
    let (v, pending) = lethal_attack(None, true, 20, 8);
    for (name, profile) in PROFILES {
        assert_eq!(blocks(profile, &v, &pending), 1, "{name}, power unread");
    }
    // And the attacker is in no zone this seat can see at all.
    let (v, pending) = lethal_attack(Some(75), false, 20, 8);
    for (name, profile) in PROFILES {
        assert_eq!(blocks(profile, &v, &pending), 1, "{name}, attacker unseen");
    }
}

/// #157. Menace is two blockers or none, and one is never an answer
/// (CR 702.111b).
///
/// The shallow path assigns one blocker per attacker by construction, so
/// `NOVICE`, `CASUAL` and `STEADY` answered a menace attacker with
/// exactly the declaration the rules forbid — and
/// `Engine::declare_blockers` refuses the whole answer rather than the
/// offending pair, so one illegal block costs every other block beside
/// it and the seat stops at the question. That was unreachable until
/// #156, which stopped `combat::can_block` asking `blockers_of(attacker)`
/// before anything is recorded: the offer now names a menace attacker
/// wherever two creatures could legally block it, so the day this pass
/// stands between three of five profiles and a stalled seat has arrived.
/// It is worth the **table**, not a point of evaluation: a refused
/// declaration is not a worse block, it is no answer at all, and for an
/// AI chair nothing is behind it. `Session::pump` passes priority when
/// the engine refuses at a `Pending::Priority`; a `ChooseBlockers` is not
/// one, so it returns without advancing that question. No clock expires
/// either — that one is for seats answering over a socket, which an AI
/// chair is not. Stall or livelock is #180's to settle.
///
/// The third position is the one that measures `search`'s own rule
/// rather than the pass added for the shallow profiles. A menace
/// rejection stayed green through an injection sweep because no
/// scenario in the suite put it in the position of *deciding*: against
/// a 4/4 with two 2/2s the search picks the gang block on its own
/// merits with the rule removed. A 2/6 against a 6/6 and a 2/2 is the
/// scene where one blocker strictly dominates — our 6/6 kills it and
/// survives, and the second creature is spent for nothing — so the
/// answer is 2 only because the leaf is refused.
#[test]
fn no_profile_answers_a_menace_attacker_with_one_blocker() {
    // Lethal, and two creatures that may block it. Blocking is not
    // optional here, so this also says the pass did not simply learn to
    // decline everything.
    let (v, pending) = menace_attack((4, 4), &[2, 2], 4);
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            2,
            "{name} answered a lethal menace attacker with the wrong \
             number of blockers"
        );
    }

    // The same attack with one creature to block with. **The engine no
    // longer offers this**: since #156 `combat::menace_satisfiable` drops
    // a menace attacker from the *offer* entirely where only one creature
    // could legally block it, so a `ChooseBlockers` naming this pairing
    // cannot arrive from a real game.
    //
    // It is kept because it measures the AI's own arithmetic, which is a
    // different question from what the engine hands over: `choose_blocks`
    // computes an answer to this shape whether or not anything presents
    // it, and a pass that only worked on offers the engine had already
    // filtered would be one nothing tested. What had to stop being
    // claimed is that this is a position a game reaches — the line above
    // said "what the rules leave", which read as a board and was one.
    let (v, pending) = menace_attack((4, 4), &[2], 4);
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            0,
            "{name} declared a block menace makes illegal, which costs \
             the whole declaration and not just this pair"
        );
    }

    // Not lethal, and one blocker is the better exchange on its own.
    let (v, pending) = menace_attack((2, 6), &[6, 2], 20);
    for (name, profile) in PROFILES {
        assert_eq!(
            blocks(profile, &v, &pending),
            2,
            "{name} took the single block that dominates the two-blocker \
             one, and it is not a legal answer"
        );
    }
}

/// #76. A card in hand is what any of its faces can be, and the land is
/// not always the face that is up.
///
/// Shatterskull Smashing is a sorcery with a land printed on its back —
/// 82 cards in this pool are that shape — so `HandObject::types` says
/// sorcery and every reading that asked it counted nought lands. The
/// engine has never agreed: `compute_legal` offers the land drop when
/// **any** face is a land (CR 712.12), so the agent was throwing away a
/// hand whose land drops the engine was about to hand it. Measured in a
/// real game before the fix: three of these beside four one-mana spells
/// was answered `MulliganTake`, and `legal.lands` on the next turn was
/// three.
#[test]
fn a_hand_whose_lands_are_on_the_back_is_not_mulliganed_as_landless() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..3)
        .map(|i| hand_card(i, "Shatterskull Smashing"))
        .chain((3..7).map(|i| hand_card(i, "Lightning Bolt")))
        .collect();
    assert!(
        v.hand.iter().all(|c| !c.types.contains(TypeSet::LAND)),
        "the premise: the view calls every one of these a spell",
    );
    let pending = Pending::Mulligan {
        player: v.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    for (name, profile) in PROFILES {
        if profile.mulligan_skill == 0 {
            continue;
        }
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::MulliganKeep,
            "{name} threw away three land drops",
        );
    }
}

/// The back face that is **not** a land drop (#152). Arguel's Blood Fast
/// turns into a land, but only by transforming: in hand it has only its
/// front face's characteristics (CR 712.8a), and CR 712.12 lets only a
/// modal card be played as its land face. So three of them beside four
/// spells is a hand with no land in it, and the engine will offer none.
/// Counting the Temple as a land kept a hand that could not play one.
#[test]
fn a_hand_whose_only_lands_are_transforming_backs_is_landless() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..3)
        .map(|i| hand_card(i, "Arguel's Blood Fast"))
        .chain((3..7).map(|i| hand_card(i, "Lightning Bolt")))
        .collect();
    let pending = Pending::Mulligan {
        player: v.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    for (name, profile) in PROFILES {
        if profile.mulligan_skill == 0 {
            continue;
        }
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::MulliganTake,
            "{name} kept a hand with no land drop in it",
        );
    }
}

/// The other half of the same count, and the reason the rule is "count
/// them" and not "notice them": seven of those cards is seven lands and
/// a hand with nothing to cast, which is a mulligan for the opposite
/// reason. A predicate that only ever added to the total would keep it.
#[test]
fn a_hand_that_is_all_back_side_lands_is_still_mulliganed() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..7)
        .map(|i| hand_card(i, "Shatterskull Smashing"))
        .collect();
    let pending = Pending::Mulligan {
        player: v.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    for (name, profile) in PROFILES {
        if profile.mulligan_skill == 0 {
            continue;
        }
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::MulliganTake,
            "{name} kept seven lands",
        );
    }
    // And a hand with no land on any face is still a mulligan, so the
    // test above is not passing because the rule stopped refusing.
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..7).map(|i| hand_card(i, "Lightning Bolt")).collect();
    for (name, profile) in PROFILES {
        if profile.mulligan_skill == 0 {
            continue;
        }
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::MulliganTake,
            "{name} kept a landless hand",
        );
    }
}

/// The same reading, at the other end of the game: a card discarded to
/// hand size is chosen by what it is worth, and a land the seat is short
/// of is worth keeping. With two land drops in hand it is a Bolt that
/// goes, not the land wearing a sorcery's face.
#[test]
fn a_back_side_land_is_not_discarded_as_a_spare_spell() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..2)
        .map(|i| hand_card(i, "Shatterskull Smashing"))
        .chain((2..5).map(|i| hand_card(i, "Lightning Bolt")))
        .collect();
    let pending = Pending::DiscardChoice {
        player: v.seat,
        count: 1,
    };
    for (name, profile) in PROFILES {
        if profile.mulligan_skill == 0 {
            continue;
        }
        let PlayerAction::ChooseObjects { objects } =
            HeuristicAgent::new(profile).act(&v, &pending)
        else {
            panic!("{name}: not a discard answer");
        };
        assert_eq!(objects.len(), 1);
        assert!(
            objects[0].slot() >= 2,
            "{name} discarded one of its two land drops",
        );
    }
}

/// #76, the half that is **not** fixed, recorded rather than claimed.
///
/// An adventure is a second castable face (CR 715), offered as
/// `CastModeKind::Face`. `filter::cast_mode` returns the `Normal`
/// position the moment one is present and never looks at a face, so
/// Brazen Borrower is always the 3/1 and Petty Theft is never cast when
/// both are affordable. That is a reading the agent does not have yet
/// rather than a rule it gets wrong — which half is better needs a value
/// model for "a creature now against a bounce now", and this crate has
/// none.
///
/// It is pinned so the day it changes is visible. Measured beside it in
/// a real game: the engine offers only the modes the pool can pay for,
/// so with two Islands it asks nothing at all and the adventure is cast
/// because it is the only option — the right face, and not a decision.
#[test]
fn an_adventure_is_offered_and_the_agent_takes_the_printed_front() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(1, "Brazen Borrower")];
    let pending = Pending::ChooseCastMode {
        player: v.seat,
        object: obj(1),
        options: vec![
            CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost: baylee_core::mana::ManaCost::ZERO.with_more_generic(3),
            },
            CastModeDesc {
                index: 1,
                kind: CastModeKind::Face(1),
                cost: baylee_core::mana::ManaCost::ZERO.with_more_generic(2),
            },
        ],
    };
    for (name, profile) in PROFILES {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::ChooseMode(0),
            "{name}: the pin moved — the agent now reads a second face, \
             so this test is the one to rewrite",
        );
    }
}

/// #223. A spell that is free while its caster controls a commander is
/// cast free, even when the printed cost is already floating.
///
/// The engine offers `Normal` only when the pool covers it, and the agent
/// took `Normal` whenever it was offered — so a Deadly Rollick beside four
/// floating mana spent all four on a spell that cost nothing. Free means
/// no mana and no other part, and the spell is the same spell either way.
/// The other two are pins, not fixes: Force of Will's alternative costs a
/// life and a blue card, and Mulldrifter's evoke gets the creature
/// sacrificed, so each keeps its printed cost when it can be paid.
#[test]
fn a_free_alternative_cost_is_taken_over_the_printed_one() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let mut v = view(0, &[20, 20], vec![permanent(obj(30), PlayerId::new(1), 2)]);
    v.hand = vec![
        hand_card(1, "Deadly Rollick"),
        hand_card(2, "Force of Will"),
        hand_card(3, "Mulldrifter"),
    ];
    for (slot, name, answer) in [
        (1, "Deadly Rollick", 1),
        (2, "Force of Will", 0),
        (3, "Mulldrifter", 0),
    ] {
        // Priced the way the engine prices them: the face's own costs.
        let face = &baylee_cards::by_index(baylee_cards::decks::by_name(name).unwrap())
            .unwrap()
            .faces[0];
        let pending = Pending::ChooseCastMode {
            player: v.seat,
            object: obj(slot),
            options: vec![
                CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: face.mana_cost,
                },
                CastModeDesc {
                    index: 1,
                    kind: CastModeKind::Alternative(0),
                    cost: face.alternative_costs[0].cost.mana,
                },
            ],
        };
        for (profile_name, profile) in PROFILES {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::ChooseMode(answer),
                "{profile_name}: {name}",
            );
        }
    }
}

#[test]
fn the_search_finds_a_menace_gang_block() {
    let mut attacker = permanent(obj(1), PlayerId::new(1), 4);
    attacker.keywords = baylee_cards_dsl::KeywordSet::MENACE.bits();
    let mut v = view(
        0,
        &[3, 20],
        vec![
            attacker,
            permanent(obj(2), PlayerId::new(0), 2),
            permanent(obj(3), PlayerId::new(0), 2),
        ],
    );
    v.combat.attackers.push(baylee_view::AttackerView {
        creature: obj(1),
        defending: Defender::Player(v.seat),
        blocked: false,
    });
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: v.seat,
        attacker: PlayerId::new(1),
        blockers: vec![
            baylee_engine::choice::BlockOption {
                blocker: obj(2),
                attackers: vec![obj(1)],
            },
            baylee_engine::choice::BlockOption {
                blocker: obj(3),
                attackers: vec![obj(1)],
            },
        ],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
        PlayerAction::DeclareBlockers {
            blockers: vec![(obj(2), obj(1)), (obj(3), obj(1))]
        }
    );
}

/// Crew (CR 702.122a) is answered by the total power the question
/// states: the strongest creatures first, by the powers the question
/// counts, until the number is reached, a creature with a negative power
/// never. A default profile answers it too, because the seat is already
/// paying the price.
#[test]
fn crew_taps_the_strongest_until_the_total_is_reached() {
    let me = PlayerId::new(0);
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), me, 1),
            permanent(obj(2), me, 3),
            permanent(obj(3), me, 1),
            permanent(obj(4), me, -1),
        ],
    );
    let ask = |power| Pending::ChooseCards {
        player: me,
        options: vec![obj(1), obj(2), obj(3), obj(4)],
        min: 1,
        max: 4,
        prompt: ChoicePrompt::CostCrew { power },
        total: Some(baylee_engine::choice::CardTotal {
            of: baylee_engine::choice::Measure::Power,
            weights: vec![1, 3, 1, -1],
            at_least: Some(i32::from(power)),
            at_most: None,
        }),
    };
    assert_eq!(
        agent().act(&v, &ask(3)),
        PlayerAction::ChooseObjects {
            objects: vec![obj(2)]
        }
    );
    assert_eq!(
        agent().act(&v, &ask(5)),
        PlayerAction::ChooseObjects {
            objects: vec![obj(2), obj(1), obj(3)]
        }
    );
}

/// Every profile holds its blocks to the bounds the blockers question
/// states, not only to the menace it reads off the view: an attacker the
/// question bounds to two or more blockers (CR 702.111b) is blocked by
/// two or by none, whatever the view shows of its keywords. With two
/// creatures offered against a lethal attacker the default profile
/// blocks with both; with one offered, nobody blocks it.
#[test]
fn every_profile_holds_its_blocks_to_the_bounds_the_question_states() {
    let me = PlayerId::new(0);
    let v = {
        let mut v = view(
            0,
            &[3, 20],
            vec![
                permanent(obj(1), PlayerId::new(1), 4),
                permanent(obj(2), me, 2),
                permanent(obj(3), me, 2),
            ],
        );
        v.combat.attackers.push(baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(v.seat),
            blocked: false,
        });
        v
    };
    let ask = |blockers: &[u32]| Pending::ChooseBlockers {
        demands: Vec::new(),
        player: me,
        attacker: PlayerId::new(1),
        blockers: blockers
            .iter()
            .map(|&b| baylee_engine::choice::BlockOption {
                blocker: obj(b),
                attackers: vec![obj(1)],
            })
            .collect(),
        bounds: vec![baylee_engine::choice::AttackerBound {
            attacker: obj(1),
            min_blockers: 2,
            max_blockers: u32::MAX,
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
    };
    for blockers in [&[2, 3][..], &[2]] {
        let pending = ask(blockers);
        for (profile_name, profile) in PROFILES {
            let answer = HeuristicAgent::new(profile).act(&v, &pending);
            assert_eq!(
                pending.answer_fault(&answer),
                None,
                "{profile_name} against {blockers:?}: {answer:?}"
            );
        }
    }
    assert_eq!(
        agent().act(&v, &ask(&[2, 3])),
        PlayerAction::DeclareBlockers {
            blockers: vec![(obj(2), obj(1)), (obj(3), obj(1))]
        },
        "three life against four power: both block"
    );
}
