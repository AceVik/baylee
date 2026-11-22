//! Wave 26c unit tests (`chains.rs`):
//! - Wave 14c generalized 2nd-target chains via `Effect::WithSecondTarget`,
//!   multi-count `second_target_up_to`, `CardInFirstTargetsGraveyard` non-shuffling moves,
//!   and `Effect::AttachSlot` (`First -> Second`, `Second -> First`, `First -> This`, `This -> First`, `This -> Second`).
//! - Wave 14e restricted inline modal choices via `Effect::ChooseOneRestricted` with
//!   `ModalRestriction::{OncePerTurn, OncePerPermanent}`.
//! - Strict Paper-MTG & soundness refusal boundaries (including keeping `TargetUnique$ True`
//!   refused on new APIs until `TargetConstraint::DistinctFromPriorTargetGroups` is wired).

use super::*;

fn w26c_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Elf".into(),
            "Goblin".into(),
            "Human".into(),
            "Kor".into(),
            "Mercenary".into(),
            "Myr".into(),
            "Noble".into(),
            "Ogre".into(),
            "Rebel".into(),
            "Robot".into(),
            "Soldier".into(),
            "Vampire".into(),
            "Warrior".into(),
            "Wizard".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Equipment".into(),
            "Food".into(),
            "Treasure".into(),
        ],
        enchantment: vec!["Aura".into()],
        land: vec![
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn w26c_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "c_a_treasure_sac",
            "Name:Treasure Token\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any",
        ),
        (
            "w_1_1_soldier",
            "Name:Soldier\nTypes:Creature Soldier\nColors:white\nPT:1/1",
        ),
    ])
}

fn read_w26c(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &w26c_cats(), Some(&w26c_tokens())).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &w26c_cats(), Some(&w26c_tokens()))
        )
    })
}

fn refused_w26c(text: &str) -> bool {
    transcode(&parse(text), &w26c_cats(), Some(&w26c_tokens())).is_none()
}

// ---------------------------------------------------------------------------
// 1. Effect::WithSecondTarget — ChangeZone multi-target chains
// ---------------------------------------------------------------------------

#[test]
fn with_second_target_change_zone_real_paper_cards() {
    // Aether Helix: bounce target permanent, return target permanent card from graveyard to hand.
    let aether_helix = read_w26c(
        "Name:Aether Helix\nManaCost:3 G U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Permanent.YouCtrl",
    );
    assert_eq!(aether_helix.abilities.len(), 1);
    let a = &aether_helix.abilities[0];
    assert!(
        a.contains("Effect::bounce(TargetSpec::Object(&Filter::Any))")
            && a.contains("Effect::WithSecondTarget(&[Effect::GraveyardToHand")
            && a.contains("targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))")
            && a.contains("second_targets = Some(TargetReq::one(TargetSpec::CardInGraveyard("),
        "{a}"
    );

    // Aether Tradewinds / Peel from Reality: bounce your permanent + bounce permanent you don't control.
    let aether_tradewinds = read_w26c(
        "Name:Aether Tradewinds\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Permanent.YouCtrl | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Permanent.YouDontCtrl",
    );
    let t = &aether_tradewinds.abilities[0];
    assert!(
        t.contains("Effect::bounce(TargetSpec::Object(")
            && t.contains("Effect::WithSecondTarget(&[Effect::bounce(TargetSpec::Object(")
            && t.contains("second_targets = Some(TargetReq::one("),
        "{t}"
    );

    // Badlands Revival: up to one graveyard creature to battlefield + up to one graveyard permanent to hand.
    let badlands_revival = read_w26c(
        "Name:Badlands Revival\nManaCost:3 B G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Creature.YouOwn | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Card.Permanent+YouOwn",
    );
    let b = &badlands_revival.abilities[0];
    assert!(
        b.contains("Effect::GraveyardToBattlefield")
            && b.contains("Effect::WithSecondTarget(&[Effect::GraveyardToHand")
            && b.contains("targets = Some(TargetReq::up_to_one(")
            && b.contains("second_targets = Some(TargetReq::up_to_one("),
        "{b}"
    );

    // Bounce + conditional bounce unless controller pays {1}:
    // WithSecondTarget MUST wrap OUTSIDE PlayerMayPayOr so ControllerOfTarget reads Slot 2!
    let tax_bounce = read_w26c(
        "Name:Taxed Tradewinds\nManaCost:U U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | ValidTgts$ Creature.YouDontCtrl | Origin$ Battlefield | Destination$ Hand | UnlessCost$ 1 | UnlessPayer$ TargetedController",
    );
    let w = &tax_bounce.abilities[0];
    assert!(
        w.contains("Effect::WithSecondTarget(&[Effect::PlayerMayPayOr { player: PlayerRel::ControllerOfTarget, mana: Amount::Fixed(1), effect: &Effect::bounce("),
        "ability: {w}"
    );
}

// ---------------------------------------------------------------------------
// 2. Effect::WithSecondTarget — DealDamage (DamageMap + DamageResolve) & mixed
// ---------------------------------------------------------------------------

#[test]
fn with_second_target_deal_damage_and_mixed_chains() {
    // Punish the Enemy: 3 to target player or planeswalker and 3 to target creature.
    let punish = read_w26c(
        "Name:Punish the Enemy\nManaCost:4 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 3 | DamageMap$ True | SubAbility$ DB1\n\
         SVar:DB1:DB$ DealDamage | NumDmg$ 3 | ValidTgts$ Creature | SubAbility$ DBDamageResolve\n\
         SVar:DBDamageResolve:DB$ DamageResolve",
    );
    let p = &punish.abilities[0];
    assert!(
        p.contains("Effect::DealDamage { amount: Amount::Fixed(3), target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER) }")
            && p.contains("Effect::WithSecondTarget(&[Effect::DealDamage { amount: Amount::Fixed(3), target: TargetSpec::Object(&Filter::CREATURE) }])")
            && p.contains("second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))"),
        "{p}"
    );

    // Hungry Flames / Lunge: 3 to target creature and 2 to target player or planeswalker.
    let hungry_flames = read_w26c(
        "Name:Hungry Flames\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | DamageMap$ True | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | NumDmg$ 2 | ValidTgts$ Player,Planeswalker | SubAbility$ DBDamageResolve\n\
         SVar:DBDamageResolve:DB$ DamageResolve",
    );
    let hf = &hungry_flames.abilities[0];
    assert!(
        hf.contains("Effect::DealDamage { amount: Amount::Fixed(3), target: TargetSpec::Object(&Filter::CREATURE) }")
            && hf.contains("Effect::WithSecondTarget(&[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER) }])")
            && hf.contains("second_targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)))"),
        "{hf}"
    );

    // Trick Shot: 6 to target creature and 2 to up to one other target creature token.
    let trick_shot = read_w26c(
        "Name:Trick Shot\nManaCost:4 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 6 | DamageMap$ True | SubAbility$ MoreDamage\n\
         SVar:MoreDamage:DB$ DealDamage | ValidTgts$ Creature.token | TargetMin$ 0 | TargetMax$ 1 | TargetUnique$ True | NumDmg$ 2 | SubAbility$ DBDamageResolve\n\
         SVar:DBDamageResolve:DB$ DamageResolve",
    );
    let ts = &trick_shot.abilities[0];
    assert!(
        ts.contains("Effect::WithSecondTarget(&[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::Object(&TARGET")
            && ts.contains("second_targets = Some(TargetReq::up_to_one(")
            && trick_shot.statics.contains("Filter::Not(&Filter::Reference)"),
        "ability: {ts}\nstatics: {}",
        trick_shot.statics
    );

    // Mixed Destroy -> DealDamage to target player (no DamageResolve required when first line isn't DealDamage).
    let destroy_and_burn = read_w26c(
        "Name:Destroy And Burn\nManaCost:3 B R\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature | SubAbility$ BurnPlayer\n\
         SVar:BurnPlayer:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 2",
    );
    let db = &destroy_and_burn.abilities[0];
    assert!(
        db.contains("Effect::destroy(TargetSpec::Object(&Filter::CREATURE))")
            && db.contains("Effect::WithSecondTarget(&[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::Player(PlayerRel::Chosen) }])")
            && db.contains("second_targets = Some(TargetReq::one(TargetSpec::AnyPlayer))"),
        "{db}"
    );

    // Yawgmoth's Vile Offering: reanimate up to 1, destroy up to 1, exile self.
    let yvo = read_w26c(
        "Name:Yawgmoth's Vile Offering\nManaCost:4 B\nTypes:Legendary Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | ValidTgts$ Creature,Planeswalker | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | ValidTgts$ Creature,Planeswalker | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Stack | Destination$ Exile",
    );
    let y = &yvo.abilities[0];
    assert!(
        y.contains("Effect::GraveyardToBattlefield")
            && y.contains(
                "Effect::DestroyTargetSlots { targets: &[TargetSlot::Second], no_regen: false }"
            )
            && y.contains("Effect::ExileSource")
            && y.contains("targets = Some(TargetReq::up_to_one(")
            && y.contains("second_targets = Some(TargetReq::up_to_one("),
        "{y}"
    );
}

// ---------------------------------------------------------------------------
// 3. Effect::WithSecondTarget — player 2nd targets, dynamic Pump, up-to-N, TargetOnly
// ---------------------------------------------------------------------------

#[test]
fn with_second_target_player_dynamic_pump_multi_count_and_target_only() {
    // Rally Maneuver: TargetMin$ 0 with omitted TargetMax$ on 2nd target + TargetUnique.
    let rally = read_w26c(
        "Name:Rally Maneuver\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | TargetUnique$ True | NumAtt$ +2 | KW$ First Strike | SubAbility$ DBPumpOther\n\
         SVar:DBPumpOther:DB$ Pump | ValidTgts$ Creature | TargetUnique$ True | TargetMin$ 0 | NumDef$ +2 | KW$ Lifelink",
    );
    let r = &rally.abilities[0];
    assert!(
        r.contains("Effect::PumpTargetSlot { slot: TargetSlot::Second")
            && r.contains("second_targets = Some(TargetReq::up_to_one(")
            && rally.statics.contains("Filter::Not(&Filter::Reference)"),
        "ability: {r}\nstatics: {}",
        rally.statics
    );

    // Dynamic X 2nd-target Pump lowers through Effect::WithSecondTarget.
    let x_pump = read_w26c(
        "Name:X Dual Pump\nManaCost:X B G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBNeg\n\
         SVar:DBNeg:DB$ Pump | ValidTgts$ Creature.YouDontCtrl | NumAtt$ -X | NumDef$ -X\n\
         SVar:X:Count$xPaid",
    );
    let xp = &x_pump.abilities[0];
    assert!(
        xp.contains("Effect::WithSecondTarget(&[Effect::PumpTarget { power: Amount::NegX, toughness: Amount::NegX")
            && xp.contains("second_targets = Some(TargetReq::one("),
        "{xp}"
    );

    // Player 2nd target: Destroy creature + target opponent discards 2 cards.
    let destroy_discard = read_w26c(
        "Name:Destroy And Discard\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature | SubAbility$ DBDisc\n\
         SVar:DBDisc:DB$ Discard | ValidTgts$ Opponent | NumCards$ 2 | Mode$ TgtChoose",
    );
    let dd = &destroy_discard.abilities[0];
    assert!(
        dd.contains("Effect::WithSecondTarget(&[Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 2 }])")
            && dd.contains("second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent))"),
        "{dd}"
    );

    // Multi-count second_target_up_to: TargetMin$ 0 | TargetMax$ 2 -> TargetReq::up_to(..., 2).
    let multi_second = read_w26c(
        "Name:Tap And Boost Two\nManaCost:2 G U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ BoostTwo\n\
         SVar:BoostTwo:DB$ PutCounter | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 2 | CounterType$ P1P1 | CounterNum$ 1",
    );
    let ms = &multi_second.abilities[0];
    assert!(
        ms.contains("Effect::TapTargetSlot(TargetSlot::First)")
            && ms.contains("Effect::WithSecondTarget(&[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }])")
            && ms.contains("second_targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2))"),
        "{ms}"
    );
}

#[test]
fn with_second_target_target_only_unique_and_first_target_graveyard() {
    // Two-target TargetOnly -> TargetOnly -> ChangeZone (bounce both target groups).
    let dual_bounce = read_w26c(
        "Name:Dual Bounce\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ Second\n\
         SVar:Second:DB$ TargetOnly | ValidTgts$ Creature.YouDontCtrl | SubAbility$ BounceBoth\n\
         SVar:BounceBoth:DB$ ChangeZone | Defined$ Targeted | Origin$ Battlefield | Destination$ Hand",
    );
    let dbb = &dual_bounce.abilities[0];
    assert!(
        dbb.contains("Effect::bounce(")
            && dbb.contains("Effect::WithSecondTarget(&[Effect::bounce(")
            && dbb.contains("second_targets = Some(TargetReq::one("),
        "{dbb}"
    );

    // TargetOnly Player -> ChangeZone Exile TargetsWithDefinedController$ ParentTarget
    // (exile up to 3 cards from target player's graveyard via CardInFirstTargetsGraveyard).
    let gy_exile = read_w26c(
        "Name:Targeted Graveyard Exile\nManaCost:B\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Player | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | TargetMin$ 0 | TargetMax$ 3 | TargetsWithDefinedController$ ParentTarget | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card",
    );
    let ge = &gy_exile.abilities[0];
    assert!(
        ge.contains(
            "Effect::WithSecondTarget(&[Effect::exile(TargetSpec::CardInFirstTargetsGraveyard("
        ) && ge.contains("targets = Some(TargetReq::one(TargetSpec::AnyPlayer))")
            && ge.contains(
                "second_targets = Some(TargetReq::up_to(TargetSpec::CardInFirstTargetsGraveyard("
            )
            && ge.contains(", 3))"),
        "{ge}"
    );

    // Shuffle$ True on 0..=3 CardInFirstTargetsGraveyard must stay refused (0 targets still shuffles per CR 701.20).
    assert!(refused_w26c(
        "Name:Krosan Reclamation Proxy\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Player | SubAbility$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | TargetMin$ 0 | TargetMax$ 3 | TargetsWithDefinedController$ ParentTarget | Origin$ Graveyard | Destination$ Library | Shuffle$ True | ValidTgts$ Card"
    ));

    // ChangeZone with TargetUnique$ True stays refused until TargetConstraint::DistinctFromPriorTargetGroups is wired.
    assert!(refused_w26c(
        "Name:Withdraw\nManaCost:U U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | ValidTgts$ Creature | TargetUnique$ True | Origin$ Battlefield | Destination$ Hand | UnlessCost$ 1 | UnlessPayer$ TargetedController"
    ));
}

// ---------------------------------------------------------------------------
// 4. Effect::AttachSlot — First->Second, Second->First, First->This, This->First
// ---------------------------------------------------------------------------

#[test]
fn attach_slot_two_target_real_paper_cards() {
    // Aura Finesse: TargetOnly Aura.YouCtrl -> Attach | Object$ ParentTarget | ValidTgts$ Creature -> Draw.
    let aura_finesse = read_w26c(
        "Name:Aura Finesse\nManaCost:U\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Aura.YouCtrl | SubAbility$ FinesseAttach\n\
         SVar:FinesseAttach:DB$ Attach | Object$ ParentTarget | ValidTgts$ Creature | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw",
    );
    let af = &aura_finesse.abilities[0];
    assert!(
        af.contains("Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::Second, optional: false }")
            && af.contains("Effect::draw(1)")
            && af.contains("second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))"),
        "{af}"
    );

    // Magnetic Theft & Brass Squire: Equipment -> Creature.
    let magnetic_theft = read_w26c(
        "Name:Magnetic Theft\nManaCost:R\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Equipment | IsCurse$ True | SubAbility$ MagneticTheft\n\
         SVar:MagneticTheft:DB$ Attach | Object$ ParentTarget | ValidTgts$ Creature",
    );
    assert!(
        magnetic_theft.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::Second, optional: false }"
        ),
        "{}",
        magnetic_theft.abilities[0]
    );

    // Kor Outfitter: optional ETB trigger attaching target Equipment to target creature you control.
    let kor_outfitter = read_w26c(
        "Name:Kor Outfitter\nManaCost:W W\nTypes:Creature Kor Soldier\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ EquipmentSelection | OptionalDecider$ You\n\
         SVar:EquipmentSelection:DB$ TargetOnly | ValidTgts$ Equipment.YouCtrl | SubAbility$ KorOutfitting\n\
         SVar:KorOutfitting:DB$ Attach | Object$ ParentTarget | ValidTgts$ Creature.YouCtrl",
    );
    assert!(
        kor_outfitter.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::Second, optional: false }"
        ) && kor_outfitter.abilities[0].contains("second_targets = Some(TargetReq::one("),
        "{}",
        kor_outfitter.abilities[0]
    );

    // Blacksmith's Talent shape: up to one target creature host + optional: true variant.
    let optional_attach = read_w26c(
        "Name:Optional Attach\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Equipment.YouCtrl | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ ParentTarget | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 1 | Optional$ True",
    );
    assert!(
        optional_attach.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::Second, optional: true }"
        ) && optional_attach.abilities[0].contains("second_targets = Some(TargetReq::up_to_one("),
        "{}",
        optional_attach.abilities[0]
    );

    // Reverse direction (Second -> First) with multi-object Equipment attachment.
    let reverse_multi = read_w26c(
        "Name:Reverse Multi Attach\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Defined$ ParentTarget | Object$ Targeted | ValidTgts$ Equipment.YouCtrl | TargetMin$ 0 | TargetMax$ 2",
    );
    assert!(
        reverse_multi.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::Second, to: TargetSlot::First, optional: false }"
        ) && reverse_multi.abilities[0].contains("second_targets = Some(TargetReq::up_to("),
        "{}",
        reverse_multi.abilities[0]
    );
}

#[test]
fn attach_slot_single_target_self_real_paper_cards() {
    // Kazuul's Toll Collector: TargetOnly Equipment.YouCtrl -> Attach | Object$ ParentTarget | Defined$ Self.
    let kazuul = read_w26c(
        "Name:Kazuul's Toll Collector\nManaCost:2 R\nTypes:Creature Ogre Warrior\nPT:3/2\n\
         A:AB$ TargetOnly | Cost$ 0 | ValidTgts$ Equipment.YouCtrl | SubAbility$ KazuulAttach | SorcerySpeed$ True\n\
         SVar:KazuulAttach:DB$ Attach | Object$ ParentTarget | Defined$ Self",
    );
    assert!(
        kazuul.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::This, optional: false }"
        ),
        "{}",
        kazuul.abilities[0]
    );

    // Cloud, Ex-SOLDIER / Sokka, Swordmaster shape: 1-line Attach | Defined$ Self | Object$ Targeted.
    let sokka_trig = read_w26c(
        "Name:Sokka Proxy\nManaCost:2 W\nTypes:Legendary Creature Human Warrior\nPT:3/3\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | ValidTgts$ Equipment.YouCtrl | TargetMin$ 0 | TargetMax$ 1 | Object$ Targeted | Defined$ Self",
    );
    assert!(
        sokka_trig.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::This, optional: false }"
        ) && sokka_trig.abilities[0].contains("targets = Some(TargetReq::up_to_one("),
        "{}",
        sokka_trig.abilities[0]
    );

    // Armory Automaton: attach any number of target Equipment to Self (MaxTargets: Count$Valid Equipment).
    let armory_automaton = read_w26c(
        "Name:Armory Automaton\nManaCost:3\nTypes:Artifact Creature Construct\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ DBAttach | OptionalDecider$ You\n\
         SVar:DBAttach:DB$ Attach | ValidTgts$ Equipment | TargetMin$ 0 | TargetMax$ MaxTargets | Object$ Targeted | Defined$ Self\n\
         SVar:MaxTargets:Count$Valid Equipment",
    );
    assert!(
        armory_automaton.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::First, to: TargetSlot::This, optional: false }"
        ) && armory_automaton.abilities[0].contains("TargetReq::up_to(")
            && armory_automaton.abilities[0].contains(", 255)"),
        "{}",
        armory_automaton.abilities[0]
    );

    // Optional AttachSelf -> Effect::AttachSlot { object: TargetSlot::This, to: TargetSlot::First, optional: true }.
    let opt_self = read_w26c(
        "Name:Optional Self Attach\nManaCost:2\nTypes:Artifact Equipment\n\
         A:AB$ Attach | Cost$ 1 | ValidTgts$ Creature.YouCtrl | Optional$ True",
    );
    assert!(
        opt_self.abilities[0].contains(
            "Effect::AttachSlot { object: TargetSlot::This, to: TargetSlot::First, optional: true }"
        ),
        "{}",
        opt_self.abilities[0]
    );
}

// ---------------------------------------------------------------------------
// 5. Effect::ChooseOneRestricted — OncePerPermanent & OncePerTurn
// ---------------------------------------------------------------------------

#[test]
fn choose_one_restricted_once_per_permanent_and_once_per_turn() {
    // Henrika Domnathi modes (OncePerPermanent / ChoiceRestriction$ ThisGame).
    let henrika_opt = read_w26c(
        "Name:Henrika Proxy\nManaCost:2 B B\nTypes:Legendary Creature Vampire\nPT:1/3\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | OptionalDecider$ You | Execute$ TrigCharm | TriggerZones$ Battlefield\n\
         SVar:TrigCharm:DB$ Charm | Choices$ SacDom,DrawDom,TransDom | ChoiceRestriction$ ThisGame\n\
         SVar:SacDom:DB$ Sacrifice | Amount$ 1 | SacValid$ Creature | Defined$ Player\n\
         SVar:DrawDom:DB$ Draw | Defined$ You | NumCards$ 1 | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ You | LifeAmount$ 1\n\
         SVar:TransDom:DB$ SetState | Defined$ Self | Mode$ Transform",
    );
    let h = &henrika_opt.abilities[0];
    assert!(
        h.contains("Effect::ChooseOneRestricted { choices: &[")
            && h.contains("Effect::Sequence(&[Effect::draw(1), Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::You }])")
            && h.contains("Effect::TransformSource")
            && h.contains("restriction: ModalRestriction::OncePerPermanent"),
        "{h}"
    );

    // Monument to Endurance modes (OncePerTurn / ChoiceRestriction$ ThisTurn).
    let monument_chain = read_w26c(
        "Name:Monument Proxy\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ Discarded | ValidCard$ Card.YouOwn | OptionalDecider$ You | TriggerZones$ Battlefield | Execute$ TrigCharm\n\
         SVar:TrigCharm:DB$ Charm | Choices$ DBDraw,DBToken,DBLoseLife | ChoiceRestriction$ ThisTurn\n\
         SVar:DBDraw:DB$ Draw\n\
         SVar:DBToken:DB$ Token | TokenScript$ c_a_treasure_sac\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 3",
    );
    let m = &monument_chain.abilities[0];
    assert!(
        m.contains("Effect::ChooseOneRestricted { choices: &[")
            && m.contains("Effect::draw(1)")
            && m.contains(
                "Effect::LoseLife { amount: Amount::Fixed(3), target: PlayerRel::Opponent }"
            )
            && m.contains("restriction: ModalRestriction::OncePerTurn"),
        "{m}"
    );

    // Galadriel, Light of Valinor modes (OncePerTurn: mana + counter-all + scry-then-draw).
    let galadriel_script = parse(
        "Name:Galadriel, Light of Valinor\nManaCost:2 G W U\nTypes:Legendary Creature Elf Noble\nPT:3/3\n\
         SVar:TrigCharm:DB$ Charm | Choices$ DBRamp,DBPutCounterAll,DBScry | ChoiceRestriction$ ThisTurn\n\
         SVar:DBRamp:DB$ Mana | Produced$ G | Amount$ 3\n\
         SVar:DBPutCounterAll:DB$ PutCounterAll | ValidCards$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 1\n\
         SVar:DBScry:DB$ Scry | ScryNum$ 2 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw",
    );
    let cats = w26c_cats();
    let mut tx = Tx::new(&galadriel_script, &cats, None);
    let mut chain = Chain::default();
    assert!(
        tx.chain(galadriel_script.svars.get("TrigCharm").unwrap(), &mut chain)
            .is_some(),
        "Galadriel TrigCharm should lower cleanly in chains.rs"
    );
    assert_eq!(chain.effects.len(), 1);
    assert!(
        chain.effects[0].contains("Effect::ChooseOneRestricted { choices: &[")
            && chain.effects[0].contains("Effect::mana(ManaColor::Green, 3)")
            && chain.effects[0].contains("Effect::Sequence(&[Effect::scry(2), Effect::draw(1)])")
            && chain.effects[0].contains("restriction: ModalRestriction::OncePerTurn"),
        "{}",
        chain.effects[0]
    );
    assert!(chain.moves_library, "Scry/Draw propagates moves_library");
    assert!(
        !chain.could_add_mana,
        "ChooseOneRestricted must never be classified as a mana ability"
    );

    // The Vision modes (OncePerTurn) via direct Tx::chain check.
    let vision_script = parse(
        "Name:The Vision\nManaCost:4\nTypes:Legendary Artifact Creature Robot\nPT:2/5\n\
         SVar:TrigCharm:DB$ Charm | Choices$ DBSolarBeam,DBDensityControl,DBTechnopathy | ChoiceRestriction$ ThisTurn\n\
         SVar:DBSolarBeam:DB$ Pump | Defined$ Self | KW$ Double Strike\n\
         SVar:DBDensityControl:DB$ Pump | Defined$ Self | KW$ Indestructible\n\
         SVar:DBTechnopathy:DB$ Draw",
    );
    let mut tx = Tx::new(&vision_script, &cats, None);
    let mut chain = Chain::default();
    assert!(
        tx.chain(vision_script.svars.get("TrigCharm").unwrap(), &mut chain)
            .is_some()
    );
    assert!(
        chain.effects[0].contains("Effect::ChooseOneRestricted")
            && chain.effects[0].contains("KeywordSet::DOUBLE_STRIKE")
            && chain.effects[0].contains("KeywordSet::INDESTRUCTIBLE")
            && chain.effects[0].contains("ModalRestriction::OncePerTurn"),
        "{}",
        chain.effects[0]
    );
}

// ---------------------------------------------------------------------------
// 6. Strict Paper-MTG & Soundness Refusals
// ---------------------------------------------------------------------------

#[test]
fn wave26c_soundness_and_non_paper_refusals() {
    // 1. 3-target chains stay cleanly refused until Chain::third_target is wired.
    assert!(refused_w26c(
        "Name:Cone of Flame\nManaCost:3 R R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 1 | DamageMap$ True | SubAbility$ DB2\n\
         SVar:DB2:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | TargetUnique$ True | SubAbility$ DB3\n\
         SVar:DB3:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | TargetUnique$ True | SubAbility$ DBRes\n\
         SVar:DBRes:DB$ DamageResolve"
    ));
    assert_eq!(
        refusal_reason(
            &parse(
                "Name:Three Distinct Targets\nManaCost:3 R R\nTypes:Sorcery\n\
                 A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 1 | SubAbility$ DB2\n\
                 SVar:DB2:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 2 | SubAbility$ DB3\n\
                 SVar:DB3:DB$ DealDamage | ValidTgts$ Planeswalker | NumDmg$ 3"
            ),
            &w26c_cats(),
            None
        )
        .as_deref(),
        Some("unsupported independent target in one chain")
    );

    // 2. Exact count > 1 on 2nd target (TargetMin$ 2 | TargetMax$ 2) must stay refused.
    assert!(refused_w26c(
        "Name:ExactSecond\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ BounceTwo\n\
         SVar:BounceTwo:DB$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Hand | TargetMin$ 2 | TargetMax$ 2"
    ));

    // 3. TargetUnique$ True on 2nd target when 1st target is multi-target must stay refused
    //    (Filter::Not(&Filter::Reference) only excludes the first object).
    assert!(refused_w26c(
        "Name:MultiFirstUniqueSecond\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ Tap | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | SubAbility$ BounceOther\n\
         SVar:BounceOther:DB$ ChangeZone | ValidTgts$ Creature | TargetUnique$ True | Origin$ Battlefield | Destination$ Hand"
    ));

    // 4. UnlessResolveSubs$ on 2nd target must stay refused.
    assert!(refused_w26c(
        "Name:SecondUnlessSubs\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ Bounce\n\
         SVar:Bounce:DB$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Hand | UnlessCost$ 1 | UnlessResolveSubs$ WhenNotPaid | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw"
    ));

    // 5. ConditionDefined$ ParentTarget on 2nd target must stay refused.
    assert!(refused_w26c(
        "Name:SecondCondParent\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ Bounce\n\
         SVar:Bounce:DB$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Hand | ConditionDefined$ ParentTarget | ConditionPresent$ Creature.tapped"
    ));

    // 6. Trailing Defined$ Targeted after 2nd target must stay refused.
    assert!(refused_w26c(
        "Name:TrailingTargeted\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ Second\n\
         SVar:Second:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ Third\n\
         SVar:Third:DB$ Untap | Defined$ Targeted"
    ));
}

#[test]
fn wave26c_attach_slot_and_restricted_modal_refusals() {
    // 7. AttachSlot soundness: Auriok Survivors shape (zone-change ChangeZone before Attach) must stay refused (CR 400.7).
    assert!(refused_w26c(
        "Name:Auriok Survivors Proxy\nManaCost:5 W\nTypes:Creature Human Soldier\nPT:4/6\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigRet\n\
         SVar:TrigRet:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Equipment.YouOwn | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ Targeted | Defined$ Self | Optional$ True"
    ));

    // 8. AttachSlot soundness: Attach to Defined$ Self on a spell has no permanent This.
    assert!(refused_w26c(
        "Name:SpellAttachSelf\nManaCost:W\nTypes:Instant\n\
         A:SP$ Attach | ValidTgts$ Equipment.YouCtrl | Object$ Targeted | Defined$ Self"
    ));

    // 9. AttachSlot soundness: multi-target host on Attach | Object$ ParentTarget must stay refused.
    assert!(refused_w26c(
        "Name:MultiHostAttach\nManaCost:W\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Equipment.YouCtrl | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ ParentTarget | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 2"
    ));

    // 10. ChooseOneRestricted soundness: targeted mode (Demonic Pact shape) must stay refused.
    assert!(refused_w26c(
        "Name:Demonic Pact Opt Proxy\nManaCost:2 B B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | OptionalDecider$ You | Execute$ TrigCharm | TriggerZones$ Battlefield\n\
         SVar:TrigCharm:DB$ Charm | Choices$ DrawPact,DrainPact | ChoiceRestriction$ ThisGame\n\
         SVar:DrawPact:DB$ Draw | NumCards$ 2\n\
         SVar:DrainPact:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 4"
    ));

    // 11. ChooseOneRestricted strict Paper-MTG: Alchemy ChoiceRestriction$ YourLastCombat must stay refused.
    assert!(refused_w26c(
        "Name:Alchemy Courier Proxy\nManaCost:2\nTypes:Artifact Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ TrigCharm\n\
         SVar:TrigCharm:DB$ Charm | Choices$ M1,M2 | ChoiceRestriction$ YourLastCombat\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2"
    ));

    // 12. ChooseOneRestricted on a spell, CharmNum$ 2, or <2 modes must stay refused.
    assert!(refused_w26c(
        "Name:RestrictedCharmNum2\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | OptionalDecider$ You | Execute$ TrigCharm | TriggerZones$ Battlefield\n\
         SVar:TrigCharm:DB$ Charm | Choices$ M1,M2 | CharmNum$ 2 | ChoiceRestriction$ ThisTurn\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2"
    ));
    assert!(refused_w26c(
        "Name:RestrictedOneMode\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | OptionalDecider$ You | Execute$ TrigCharm | TriggerZones$ Battlefield\n\
         SVar:TrigCharm:DB$ Charm | Choices$ M1 | ChoiceRestriction$ ThisGame\n\
         SVar:M1:DB$ Draw | NumCards$ 1"
    ));
}
