use super::{mix, priced};
use baylee_cards_dsl::{Cost, CostPart};

mod mana_effects;

#[test]
fn color_planning_does_not_overflow_a_full_pool_counter() {
    use baylee_core::mana::ManaColor;
    let pool = baylee_view::ManaPoolView {
        blue: u32::MAX,
        ..Default::default()
    };
    assert!(super::one_more(pool, ManaColor::Blue).is_none());
    let after = super::one_more(pool, ManaColor::Red).unwrap();
    assert_eq!(after.blue, u32::MAX);
    assert_eq!(after.red, 1);
}

/// **A land on the back of a card is still a land the engine offers.**
///
/// A `CardIdentity` in hand names the face that is up, and for a modal
/// double-faced card that face is the spell — so every reader asking
/// `types.contains(LAND)` of it read Shatterskull Smashing as a sorcery
/// and nothing else, while `compute_legal` was already offering the land
/// drop (CR 712.12). An agent that disagrees with the offer it is
/// answering declines land drops the engine is making it.
///
/// So the rule is read over the **whole pool** rather than over an
/// example, and in two populations, because the back face is a land drop
/// for one kind of double-faced card and not the other. A modal card's
/// land back is (CR 712.12); a transforming card's is not, since in hand
/// it has only its front face's characteristics (CR 712.8a) — Arguel's
/// Blood Fast is an enchantment there, and its Temple is reached only by
/// turning over. `castable_from_hand` is what tells the two apart, and
/// `xtask validate` holds it against Scryfall's `layout`.
///
/// Both populations carry a floor and a card pinned by name, so an
/// answer that is "yes" for every back land, or "no" for every one, is
/// red here rather than a quiet pass over half the pool.
#[test]
fn a_land_on_a_modal_back_is_playable_and_one_on_a_transforming_back_is_not() {
    use baylee_core::ids::PrintRef;
    use baylee_core::types::TypeSet;

    let identity = |index| super::CardIdentity {
        index,
        print: PrintRef::new(0),
        face: 0,
    };
    let by_oracle = |oracle_id| {
        identity(
            baylee_cards::by_oracle_id(oracle_id)
                .expect("registry contains the card")
                .index,
        )
    };
    let mut read = 0usize;
    let mut modal = Vec::new();
    let mut transforming = Vec::new();
    let mut wrong = Vec::new();
    for def in baylee_cards::all() {
        read += 1;
        // Face 0, because that is what a card in hand shows and what
        // every front-face reader was looking at.
        let card = identity(def.index);
        let front_is_a_land = def.faces[0].types.contains(TypeSet::LAND);
        let back_land = def
            .faces
            .iter()
            .skip(1)
            .find(|f| f.types.contains(TypeSet::LAND));
        let playable = match back_land {
            _ if front_is_a_land => true,
            Some(back) if back.castable_from_hand => {
                modal.push(def.name());
                true
            }
            Some(_) => {
                transforming.push(def.name());
                false
            }
            None => false,
        };
        if super::plays_as_land(card) != playable {
            wrong.push(def.name());
        }
        if playable {
            assert!(
                super::land_face(card).is_some_and(|f| f.types.contains(TypeSet::LAND)),
                "{} names a face that is not the land",
                def.name()
            );
        }
    }

    assert!(
        read > 2_500,
        "read {read} cards out of the pool, which is not the pool"
    );
    assert!(
        wrong.is_empty(),
        "{} card(s) disagree with CR 712.12 / 712.8a: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(10)]
    );
    // Measured 2026-09-24: 50 modal and 32 transforming, over 2716
    // cards. Floors under both, because the pool only grows, and a
    // sweep that read none of either would report the same clean result
    // as one that read all of them.
    assert!(
        modal.len() >= 48,
        "only {} modal card(s) print a spell over a land: {modal:?}",
        modal.len()
    );
    assert!(
        transforming.len() >= 30,
        "only {} transforming card(s) turn into a land: {transforming:?}",
        transforming.len()
    );

    // Pinned by name, one of each kind.
    let shatterskull = by_oracle("78301998-fd9b-4cd5-afad-dbcb43cac2a7");
    assert!(super::plays_as_land(shatterskull), "Shatterskull Smashing");
    let blood_fast = by_oracle("be2a4bc4-8af6-48c5-9421-32d26272e71a");
    assert!(!super::plays_as_land(blood_fast), "Arguel's Blood Fast");
    assert!(super::land_face(blood_fast).is_none());

    // And a card with no land face anywhere is not a land drop.
    let bolt = by_oracle("4457ed35-7c10-48c8-9776-456485fdf070");
    assert!(!super::plays_as_land(bolt));
    assert!(super::land_face(bolt).is_none());
}

/// What a tap costs is part of the ranking. `mana_shape` has already
/// refused a mana price, so what is left is `cost.parts`, and `{T}` is
/// the only part a mana source may carry for free — a `matches!` on
/// that one variant rather than a list of the expensive ones, so a
/// `CostPart` added tomorrow is priced by default.
///
/// The bug it closes is Havenwood Battleground, which prints `{T}: Add
/// {G}` beside `{T}, Sacrifice this land: Add {G}{G}`: ranked on mana
/// made, the sacrifice won and the agent sold the land for a mana it
/// already had.
#[test]
fn only_tapping_the_permanent_is_free() {
    assert!(!priced(&Cost::TAP));
    assert!(!priced(&Cost::FREE), "an ability with no cost at all");
    assert!(priced(&baylee_cards_dsl::cost!(TapSelf, SacrificeSelf)));
    assert!(
        priced(&baylee_cards_dsl::cost!(TapSelf, PayLife(1))),
        "Spire of Industry pays the life whenever the plan asks it to"
    );
    assert!(
        priced(&baylee_cards_dsl::cost!(UntapSelf)),
        "untapping a permanent is a different button from tapping it, \
         whatever mana comes out"
    );
    assert!(priced(&baylee_cards_dsl::cost!(
        TapSelf,
        Discard(&baylee_cards_dsl::Filter::CREATURE)
    )));
}

/// The noise an agent breaks a tie with is `SplitMix`'s integer
/// finalizer: fixed arithmetic, not a platform hasher and not a process
/// RNG. A replay has to reach the same decision on another machine and
/// in another build, so the values are written down rather than merely
/// asserted to be stable within one run — a `DefaultHasher` passes
/// every property test this could ask and changes these four numbers.
#[test]
fn the_tie_break_noise_is_arithmetic_and_not_a_hasher() {
    assert_eq!(mix(0), 0);
    assert_eq!(mix(1), 6_238_072_747_940_578_789);
    assert_eq!(mix(2), 15_839_785_061_582_574_730);
    assert_eq!(mix(0xdead_beef), 5_622_224_078_331_092_714);
}

/// And it separates neighbours, which is the whole job: the noise is
/// keyed by the offered object as well as the view, so two objects one
/// apart must not sort together.
#[test]
fn neighbouring_keys_do_not_share_a_tie_break() {
    let noise: Vec<u64> = (0..64).map(mix).collect();
    let mut sorted = noise.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), noise.len(), "64 keys, 64 distinct answers");
    assert_ne!(noise[1] >> 60, noise[2] >> 60, "and they differ up top");
}

/// One part of `priced`'s contract that the `matches!` states and
/// nothing else would: a cost made only of parts it has never seen is
/// priced, not free.
#[test]
fn an_unrecognised_part_is_priced_rather_than_ignored() {
    const PARTS: [&[CostPart]; 3] = [
        &[CostPart::ExileSelf],
        &[CostPart::DiscardSelf],
        &[CostPart::ReturnSelfToHand],
    ];
    for parts in PARTS {
        let cost = Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts,
        };
        assert!(priced(&cost), "{parts:?}");
    }
}

fn test_identity(name: &str) -> super::CardIdentity {
    let index = baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("missing card {name}"));
    super::CardIdentity {
        index,
        print: baylee_core::ids::PrintRef::new(0),
        face: 0,
    }
}

fn test_public_object(
    id: u32,
    name: &str,
    controller: u8,
    attached_to: Option<u32>,
) -> baylee_view::PublicObject {
    let card = test_identity(name);
    let def = baylee_cards::by_index(card.index).expect("card exists");
    let face = &def.faces[0];
    let mut subtypes = baylee_core::types::SubtypeSet::EMPTY;
    for &s in face.subtypes {
        subtypes.insert(s);
    }
    baylee_view::PublicObject {
        battle_protector: None,
        id: super::ObjectId::new(id, 0),
        card: Some(card),
        rules: Some(card.into()),
        prepared_spell: None,
        name: face.name.into(),
        controller: baylee_core::ids::PlayerId::new(controller),
        owner: baylee_core::ids::PlayerId::new(controller),
        commander: false,
        status: baylee_view::ObjectStatus::default(),
        types: face.types,
        supertypes: face.supertypes,
        subtypes,
        chosen_subtype: None,
        chosen_name: None,
        chosen_opponent: None,
        unlocked_doors: None,
        suspended: false,
        token: None,
        colors: face.mana_cost.colors().union(face.color_indicator),
        mana_value: face.mana_cost.cmc(),
        keywords: def.keywords_for_face(0).bits(),
        power: face.power,
        toughness: face.toughness,
        base_power: face.power,
        base_toughness: face.toughness,
        loyalty: face.loyalty,
        damage: 0,
        counters: Vec::new(),
        attached_to: attached_to.map(|t| super::ObjectId::new(t, 0)),
        targets: Vec::new(),
        stack_item: None,
        times_kicked: 0,
        bought_back: false,
        summoning_sick: false,
        ability_mana_spending: baylee_core::mana::ManaSpending::EXACT,
        grants: Vec::new(),
        granted_mana: None,
        board_mana: None,
        flashback: None,
        ability_sources: Vec::new(),
        word_changes: Vec::new(),
    }
}

fn base_view() -> super::PlayerView {
    use baylee_core::ids::{PlayerId, SeatSet};
    let seat_view = |p: u8| baylee_view::SeatView {
        player: PlayerId::new(p),
        life: 20,
        poison: 0,
        energy: 0,
        speed: 0,
        rad: 0,
        hand_count: 0,
        no_max_hand_size: false,
        library_count: 40,
        graveyard_count: 0,
        loss: None,
        house_answered: None,
        mana_pool: baylee_view::ManaPoolView::default(),
        commanders: Vec::new(),
        commander_damage: Vec::new(),
    };
    super::PlayerView {
        ballots: Vec::new(),
        damage_sources: Vec::new(),
        target_objects: Vec::new(),
        seq: 1,
        seat: PlayerId::new(0),
        turn: 1,
        phase: baylee_view::Phase::FirstMain,
        step: baylee_view::Step::Main,
        active: PlayerId::new(0),
        awaiting: Some(PlayerId::new(0)),
        deciding: SeatSet::new(),
        decision_remaining_ms: None,
        clocks: Vec::new(),
        lost: Vec::new(),
        priority_held: false,
        policy_acts: Vec::new(),
        monarch: None,
        day_night: None,
        seats: vec![seat_view(0), seat_view(1)],
        hand: Vec::new(),
        shared_hands: Vec::new(),
        controlled_hands: Vec::new(),
        decision_player: None,
        hand_shared_with: SeatSet::new(),
        hand_requests: SeatSet::new(),
        hand_requested: SeatSet::new(),
        battlefield: Vec::new(),
        stack: Vec::new(),
        graveyards: vec![Vec::new(), Vec::new()],
        exile: vec![Vec::new(), Vec::new()],
        command: vec![Vec::new(), Vec::new()],
        combat: baylee_view::CombatView::default(),
        looking_at: Vec::new(),
        trigger_order: Vec::new(),
        library_tops: Vec::new(),
        owed: None,
        targeting: None,
        casting: None,
        announced_cast: None,
        announced_x: None,
        sorcery_lock: None,
        sorceries_have_flash: false,
        flash_types: baylee_core::types::TypeSet::EMPTY,
        controls: Vec::new(),
        foreign_targets: Vec::new(),
    }
}

#[test]
fn spell_mana_increase_extracts_colored_pips_from_derelor_and_respects_engine_cap() {
    use baylee_core::mana::ManaCost;
    use baylee_engine::choice::{CastModeKind, LegalActions};

    let spell_id = super::ObjectId::new(50, 0);
    let dark_ritual = test_identity("Dark Ritual");
    let lightning_bolt = test_identity("Lightning Bolt");
    let mut view = base_view();
    view.battlefield
        .push(test_public_object(10, "Derelor", 0, None));

    // 1. Controlled Derelor taxes black spells by {B}, not {1} generic.
    let mut legal = LegalActions::default();
    legal
        .spell_increases
        .push((spell_id, CastModeKind::Normal, 1));
    assert_eq!(
        super::spell_mana_increase(&view, spell_id, dark_ritual, &legal, CastModeKind::Normal),
        ManaCost::parse("{B}")
    );

    // 2. Non-black spell with generic tax {1} stays {1} generic even with Derelor out.
    assert_eq!(
        super::spell_mana_increase(
            &view,
            spell_id,
            lightning_bolt,
            &legal,
            CastModeKind::Normal
        ),
        ManaCost::parse("{1}")
    );

    // 3. Controlled Derelor ({B}) + generic tax of {2} (total = 3) produces {2}{B}.
    legal.spell_increases = vec![(spell_id, CastModeKind::Normal, 3)];
    assert_eq!(
        super::spell_mana_increase(&view, spell_id, dark_ritual, &legal, CastModeKind::Normal),
        ManaCost::parse("{2}{B}")
    );

    // 4. Opponent's Derelor does not match `Filter::YOUR_BLACK` for seat 0.
    view.battlefield[0].controller = baylee_core::ids::PlayerId::new(1);
    legal.spell_increases = vec![(spell_id, CastModeKind::Normal, 1)];
    assert_eq!(
        super::spell_mana_increase(&view, spell_id, dark_ritual, &legal, CastModeKind::Normal),
        ManaCost::parse("{1}")
    );

    // 5. Face-down disguise/morph casts are colorless (CR 708.2) and never get {B}.
    view.battlefield[0].controller = baylee_core::ids::PlayerId::new(0);
    legal.spell_increases = vec![(spell_id, CastModeKind::Disguise, 1)];
    assert_eq!(
        super::spell_mana_increase(&view, spell_id, dark_ritual, &legal, CastModeKind::Disguise),
        ManaCost::parse("{1}")
    );
}

#[test]
fn apply_triggered_mana_bonus_adds_mana_from_wild_growth_and_mana_flare() {
    use baylee_client_core::manaplan::{Source, Tap};
    use baylee_core::mana::ManaColor;

    let mut view = base_view();
    let forest_1 = test_public_object(1, "Forest", 0, None);
    let forest_2 = test_public_object(2, "Forest", 0, None);
    let mountain = test_public_object(3, "Mountain", 0, None);
    let elves = test_public_object(4, "Llanowar Elves", 0, None);
    // Wild Growth attached to Mountain #3.
    let wild_growth = test_public_object(10, "Wild Growth", 0, Some(3));
    view.battlefield = vec![
        forest_1.clone(),
        forest_2.clone(),
        mountain.clone(),
        elves.clone(),
        wild_growth,
    ];

    // Unattached Forest #1 gets no bonus from Wild Growth on Mountain #3.
    let mut s_forest = Source::fixed(forest_1.id, Tap::Intrinsic, ManaColor::Green);
    super::apply_triggered_mana_bonus(&view, &forest_1, &mut s_forest);
    assert_eq!(s_forest.amount, 1);
    assert_eq!(s_forest.colors, vec![ManaColor::Green]);

    // Enchanted Mountain #3 gets +{G}, turning {R} into a 2-mana bundle [Red, Green].
    let mut s_mountain = Source::fixed(mountain.id, Tap::Intrinsic, ManaColor::Red);
    super::apply_triggered_mana_bonus(&view, &mountain, &mut s_mountain);
    assert_eq!(s_mountain.amount, 2);
    assert!(s_mountain.bundle);
    assert_eq!(s_mountain.colors, vec![ManaColor::Red, ManaColor::Green]);

    // Adding Mana Flare doubles land mana (+1 of the type produced) but not Llanowar Elves.
    view.battlefield
        .push(test_public_object(11, "Mana Flare", 1, None));
    let mut s_forest_flare = Source::fixed(forest_2.id, Tap::Intrinsic, ManaColor::Green);
    super::apply_triggered_mana_bonus(&view, &forest_2, &mut s_forest_flare);
    assert_eq!(s_forest_flare.amount, 2);

    let mut s_elves = Source::fixed(elves.id, Tap::Ability(0), ManaColor::Green);
    super::apply_triggered_mana_bonus(&view, &elves, &mut s_elves);
    assert_eq!(
        s_elves.amount, 1,
        "Mana Flare only triggers when a land is tapped for mana"
    );
}

#[test]
fn taps_set_off_a_trigger_and_tap_destroys_or_locks_distinguish_cursed_and_clean_sources() {
    use baylee_engine::choice::LegalActions;

    let mut view = base_view();
    let cursed_forest = test_public_object(1, "Forest", 0, None);
    let clean_forest = test_public_object(2, "Forest", 0, None);
    let island = test_public_object(3, "Island", 0, None);
    let mana_vault = test_public_object(4, "Mana Vault", 0, None);
    // Blight attached to Forest #1; Wild Growth attached to Forest #2.
    let blight = test_public_object(10, "Blight", 1, Some(1));
    let wild_growth = test_public_object(11, "Wild Growth", 0, Some(2));
    view.battlefield = vec![
        cursed_forest.clone(),
        clean_forest.clone(),
        island.clone(),
        mana_vault.clone(),
        blight,
        wild_growth,
    ];

    assert!(
        super::taps_set_off_a_trigger(&view, &cursed_forest),
        "Blight on Forest #1 sets off a trigger when Forest #1 becomes tapped"
    );
    assert!(
        super::tap_destroys_or_locks(&view, &cursed_forest),
        "Blight destroys enchanted land when tapped"
    );
    assert!(
        !super::taps_set_off_a_trigger(&view, &clean_forest),
        "Wild Growth is a triggered mana ability (CR 605.1b), not a stack trigger, and Blight is on Forest #1"
    );
    assert!(!super::tap_destroys_or_locks(&view, &clean_forest));
    assert!(
        super::tap_destroys_or_locks(&view, &mana_vault),
        "Mana Vault has DoesNotUntap so tapping locks it"
    );

    // Lifetap controlled by opponent (seat 1) triggers when seat 0 taps a Forest,
    // but NOT when seat 0 taps an Island, and NOT when Lifetap is controlled by seat 0.
    let mut lifetap = test_public_object(12, "Lifetap", 1, None);
    view.battlefield.push(lifetap.clone());
    assert!(super::taps_set_off_a_trigger(&view, &clean_forest));
    assert!(!super::taps_set_off_a_trigger(&view, &island));
    view.battlefield.last_mut().unwrap().controller = baylee_core::ids::PlayerId::new(0);
    lifetap.controller = baylee_core::ids::PlayerId::new(0);
    assert!(
        !super::taps_set_off_a_trigger(&view, &clean_forest),
        "own Lifetap only triggers on an opponent's Forest"
    );

    // Verify `sources` ranks the clean Forest ahead of the Blighted Forest.
    let legal = LegalActions {
        mana_abilities: vec![cursed_forest.id, clean_forest.id],
        ..LegalActions::default()
    };
    let ranked = super::sources(&view, &legal);
    let c1 = ranked.iter().find(|s| s.id == cursed_forest.id).unwrap();
    let c2 = ranked.iter().find(|s| s.id == clean_forest.id).unwrap();
    assert!(c1.priced && !c2.priced);
    assert!(c1.preserve >= c2.preserve + 15_000);
}

#[test]
fn pays_tax_declines_untap_self_payment_when_source_is_already_untapped() {
    use baylee_cards_dsl::{Amount, Effect, PlayerRel};
    use baylee_core::mana::ManaCost;

    static UNTAP_EFFECTS: &[Effect] = &[Effect::PlayerMayPayThen {
        player: PlayerRel::You,
        mana: Amount::Fixed(4),
        effects: &[Effect::UntapSelf],
    }];

    let mut view = base_view();
    let mut vault = test_public_object(1, "Mana Vault", 0, None);
    let f1 = test_public_object(2, "Forest", 0, None);
    let f2 = test_public_object(3, "Forest", 0, None);
    let f3 = test_public_object(4, "Forest", 0, None);
    let f4 = test_public_object(5, "Forest", 0, None);
    view.battlefield = vec![vault.clone(), f1, f2, f3, f4];

    let ctx = baylee_engine::engine::DecisionContext {
        source: Some(vault.id),
        effects: UNTAP_EFFECTS,
        ..Default::default()
    };

    // Untapped Mana Vault: never pay {4} to untap an already-untapped permanent.
    assert!(!super::pays_tax(&view, ManaCost::parse("{4}"), &ctx));

    // Tapped Mana Vault with 4 untapped Forests: pay {4} to untap it.
    vault.status = baylee_view::ObjectStatus::TAPPED;
    view.battlefield[0] = vault;
    assert!(super::pays_tax(&view, ManaCost::parse("{4}"), &ctx));
}

#[test]
fn pays_tax_reads_an_any_player_payments_unpaid_effects() {
    use baylee_cards_dsl::Effect;
    use baylee_core::mana::ManaCost;

    static UPKEEP: &[Effect] = &[Effect::AnyPlayerMayPayManaOr {
        cost: ManaCost::parse("{3}"),
        effects: &[Effect::SacrificeSelf],
    }];
    static NO_COST: &[Effect] = &[Effect::AnyPlayerMayPayManaOr {
        cost: ManaCost::parse("{3}"),
        effects: &[Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Fixed(1),
        }],
    }];

    let mut view = base_view();
    let upkeep = baylee_engine::engine::DecisionContext {
        effects: UPKEEP,
        ..Default::default()
    };
    assert!(!super::pays_tax(&view, ManaCost::parse("{3}"), &upkeep));
    view.battlefield = vec![
        test_public_object(1, "Forest", 0, None),
        test_public_object(2, "Forest", 0, None),
        test_public_object(3, "Forest", 0, None),
    ];
    assert!(
        super::pays_tax(&view, ManaCost::parse("{3}"), &upkeep),
        "three available mana keep the permanent whose sacrifice nobody paying would cause"
    );

    let no_cost = baylee_engine::engine::DecisionContext {
        effects: NO_COST,
        ..Default::default()
    };
    assert!(
        !super::pays_tax(&view, ManaCost::parse("{3}"), &no_cost),
        "an unrelated unpaid effect does not become an upkeep payment"
    );
}

/// Icy Prison's upkeep asks every seat, and refusing costs a card only for its
/// controller: the mana is spent to keep this seat's own Prison, never an
/// opponent's, which may well be holding this seat's creature.
#[test]
fn pays_tax_keeps_only_its_own_permanent_when_any_player_may_pay() {
    use baylee_cards_dsl::Effect;
    use baylee_core::mana::ManaCost;

    static UPKEEP: &[Effect] = &[Effect::AnyPlayerMayPayManaOr {
        cost: ManaCost::parse("{3}"),
        effects: &[Effect::SacrificeSelf],
    }];
    let prison = test_public_object(4, "Icy Prison", 1, None);
    let source = prison.id;
    let mut view = base_view();
    view.battlefield = vec![
        test_public_object(1, "Forest", 0, None),
        test_public_object(2, "Forest", 0, None),
        test_public_object(3, "Forest", 0, None),
        prison,
    ];
    let context = baylee_engine::engine::DecisionContext {
        source: Some(source),
        effects: UPKEEP,
        ..Default::default()
    };
    assert!(
        !super::pays_tax(&view, ManaCost::parse("{3}"), &context),
        "an opponent's Prison is not this seat's to keep"
    );

    view.battlefield[3].controller = baylee_core::ids::PlayerId::new(0);
    assert!(
        super::pays_tax(&view, ManaCost::parse("{3}"), &context),
        "the same payment keeps this seat's own Prison"
    );
}

/// Power Sink's "unless" carries a sequence that starts with the counter, and
/// refusing it loses the spell: paid when it can be, `{0}` included (NEW-15).
/// The effects are the card's own, read from the registry.
#[test]
fn pays_tax_pays_power_sinks_x_to_keep_the_spell() {
    use baylee_cards_dsl::AbilityDef;
    use baylee_core::mana::ManaCost;

    let sink = baylee_cards::by_index(test_identity("Power Sink").index).expect("Power Sink");
    let Some(AbilityDef::Spell { effects, .. }) = sink.abilities.first() else {
        panic!("Power Sink is one spell ability");
    };
    let ctx = baylee_engine::engine::DecisionContext {
        effects,
        ..Default::default()
    };
    let mut view = base_view();
    assert!(
        super::pays_tax(&view, ManaCost::ZERO, &ctx),
        "X = 0 is free"
    );
    assert!(
        !super::pays_tax(&view, ManaCost::parse("{2}"), &ctx),
        "nothing to pay {{2}} with"
    );
    view.battlefield = vec![
        test_public_object(1, "Forest", 0, None),
        test_public_object(2, "Forest", 0, None),
    ];
    assert!(
        super::pays_tax(&view, ManaCost::parse("{2}"), &ctx),
        "two Forests pay X = 2"
    );
}

#[test]
fn spell_can_be_cast_now_checks_the_engines_word_then_conditions_and_mandatory_costs() {
    let spell_id = super::ObjectId::new(99, 0);
    let sirens_call = test_identity("Siren's Call");
    let blizzard = test_identity("Blizzard");
    let tidal_influence = test_identity("Tidal Influence");
    let terror = test_identity("Terror");
    let sacrifice = test_identity("Sacrifice");

    let mut view = base_view();
    // The engine would cast the card once it is paid for, so what the rest
    // of this asks is only what the view can read.
    let legal = baylee_engine::choice::LegalActions {
        payable: vec![spell_id],
        ..Default::default()
    };

    // 1. Siren's Call: only during opponent's turn before DeclareAttackers.
    assert!(
        !super::spell_can_be_cast_now(&view, &legal, spell_id, sirens_call),
        "cannot cast Siren's Call on own turn"
    );
    view.active = baylee_core::ids::PlayerId::new(1);
    view.phase = baylee_view::Phase::FirstMain;
    view.step = baylee_view::Step::Main;
    assert!(
        super::spell_can_be_cast_now(&view, &legal, spell_id, sirens_call),
        "can cast Siren's Call on opponent's FirstMain before DeclareAttackers"
    );
    view.phase = baylee_view::Phase::Combat;
    view.step = baylee_view::Step::DeclareAttackers;
    assert!(
        !super::spell_can_be_cast_now(&view, &legal, spell_id, sirens_call),
        "cannot cast Siren's Call once DeclareAttackers has begun"
    );

    // 2. Blizzard: only if you control a snow land.
    view.active = baylee_core::ids::PlayerId::new(0);
    view.phase = baylee_view::Phase::FirstMain;
    view.step = baylee_view::Step::Main;
    view.battlefield = vec![test_public_object(1, "Forest", 0, None)];
    assert!(
        !super::spell_can_be_cast_now(&view, &legal, spell_id, blizzard),
        "cannot cast Blizzard with only a non-snow Forest"
    );
    view.battlefield
        .push(test_public_object(2, "Snow-Covered Forest", 0, None));
    assert!(
        super::spell_can_be_cast_now(&view, &legal, spell_id, blizzard),
        "can cast Blizzard when controlling a Snow-Covered Forest"
    );

    // 3. Tidal Influence: only if no permanent named Tidal Influence is on the battlefield.
    assert!(super::spell_can_be_cast_now(
        &view,
        &legal,
        spell_id,
        tidal_influence
    ));
    view.battlefield
        .push(test_public_object(3, "Tidal Influence", 1, None));
    assert!(
        !super::spell_can_be_cast_now(&view, &legal, spell_id, tidal_influence),
        "cannot cast Tidal Influence when one is already on the battlefield"
    );

    // 4. Terror over empty battlefields (NEW-04): whether it has a legal
    // target is the engine's to say, and a card it lists in neither
    // `castable` nor `payable` is not tapped for, whatever the view holds.
    view.battlefield.clear();
    assert!(
        !super::spell_can_be_cast_now(
            &view,
            &baylee_engine::choice::LegalActions::default(),
            spell_id,
            terror
        ),
        "Terror with nothing to destroy is not castable now"
    );
    assert!(super::spell_can_be_cast_now(
        &view, &legal, spell_id, terror
    ));

    // 5. Sacrifice: mandatory sacrifice of a creature you control.
    view.battlefield = vec![test_public_object(30, "Grizzly Bears", 1, None)];
    assert!(
        !super::spell_can_be_cast_now(&view, &legal, spell_id, sacrifice),
        "opponent's creature cannot pay your mandatory sacrifice cost"
    );
    view.battlefield
        .push(test_public_object(31, "Grizzly Bears", 0, None));
    assert!(super::spell_can_be_cast_now(
        &view, &legal, spell_id, sacrifice
    ));
}
