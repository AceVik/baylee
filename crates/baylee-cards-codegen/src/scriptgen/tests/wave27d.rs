//! Codegen Wave 27d unit tests: `Modifier::ActivatedAbilitiesCostLess`,
//! `Modifier::FirstSpellEachTurnCostAdjust`, `Modifier::MustAttackPlayer`,
//! `Filter::Saddled`, and owner exile play permission
//! (`ExilePermission::caster` / `PlayerRel::OwnerOfTarget`), plus honest
//! refusals for unsupported Forge parameters and digital/Alchemy mechanics.

use super::*;

fn w27d_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Beast".into(),
            "Bird".into(),
            "Camel".into(),
            "Cat".into(),
            "Dragon".into(),
            "Elephant".into(),
            "Goblin".into(),
            "Horse".into(),
            "Human".into(),
            "Mount".into(),
            "Robot".into(),
            "Siren".into(),
            "Soldier".into(),
            "Warlock".into(),
            "Wizard".into(),
            "Wolf".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Equipment".into(),
            "Food".into(),
            "Treasure".into(),
            "Vehicle".into(),
        ],
        enchantment: vec![
            "Aura".into(),
            "Background".into(),
            "Class".into(),
            "Saga".into(),
        ],
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

fn read_w27d(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w27d_cats();
    transcode(&parsed, &cats, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, None)
        )
    })
}

fn refused_w27d(text: &str) -> bool {
    let parsed = parse(text);
    transcode(&parsed, &w27d_cats(), None).is_none()
}

// =========================================================================
// 1. Modifier::ActivatedAbilitiesCostLess (Agatha, Boom Scholar, etc.)
// =========================================================================

#[test]
fn activated_abilities_cost_less_agatha_and_boom_scholar() {
    // Agatha of the Vile Cauldron: dynamic Count$CardPower + MinMana$ 1.
    let agatha = read_w27d(
        "Name:Agatha of the Vile Cauldron\nManaCost:R G\nTypes:Legendary Creature Human Warlock\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.YouCtrl | Type$ Ability | Amount$ X | MinMana$ 1 | AffectedZone$ Battlefield\n\
         A:AB$ PumpAll | Cost$ 4 R G | ValidCards$ Creature.YouCtrl+StrictlyOther | NumAtt$ +1 | NumDef$ +1 | KW$ Trample & Haste\n\
         SVar:X:Count$CardPower",
    );
    assert!(
        agatha.abilities.iter().any(|a| {
            a.contains("Modifier::ActivatedAbilitiesCostLess")
                && a.contains("filter: &Filter::YOUR_CREATURE")
                && a.contains("amount: Amount::SourcePower")
                && a.contains("min_mana: 1")
                && a.contains("exhaust_only: false")
        }),
        "{:?}",
        agatha.abilities
    );

    // Boom Scholar: ValidSpell$ Activated.Exhaust -> exhaust_only: true.
    let boom = read_w27d(
        "Name:Boom Scholar\nManaCost:1 R G\nTypes:Creature Goblin Advisor\nPT:3/3\n\
         S:Mode$ ReduceCost | ValidCard$ Permanent.YouCtrl+Other | ValidSpell$ Activated.Exhaust | Amount$ 2 | AffectedZone$ Battlefield\n\
         A:AB$ PumpAll | Cost$ 4 R G | ValidCards$ Creature.YouCtrl,Vehicle.YouCtrl | KW$ Trample | Exhaust$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2",
    );
    assert!(
        boom.abilities.iter().any(|a| {
            a.contains("Modifier::ActivatedAbilitiesCostLess")
                && a.contains("amount: Amount::Fixed(2)")
                && a.contains("min_mana: 0")
                && a.contains("exhaust_only: true")
        }),
        "{:?}",
        boom.abilities
    );
    assert!(
        boom.statics.contains("COST_ABILITY") && boom.statics.contains("Filter::Another"),
        "{:?}",
        boom.statics
    );

    // Dynamic count-based ability reduction in graveyard.
    let gy_dyn = read_w27d(
        "Name:GraveyardTuner\nManaCost:3\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.YouOwn+inZoneGraveyard | Type$ Ability | Amount$ X | MinMana$ 1\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
    );
    assert!(
        gy_dyn.abilities.iter().any(|a| {
            a.contains("Modifier::ActivatedAbilitiesCostLess")
                && a.contains("Amount::CountOf")
                && a.contains("min_mana: 1")
                && a.contains("exhaust_only: false")
        }),
        "{:?}",
        gy_dyn.abilities
    );
    assert!(
        gy_dyn
            .statics
            .contains("Filter::InZone(ZoneRef::Graveyard)"),
        "{:?}",
        gy_dyn.statics
    );
}

#[test]
fn activated_abilities_cost_less_honest_refusals() {
    for (label, script) in [
        (
            "non-mana-ability exclusion (Zirda) not modeled in ActivatedAbilitiesCostLess",
            "Name:Zirda\nManaCost:1 R W\nTypes:Legendary Creature\nPT:3/3\n\
             S:Mode$ ReduceCost | ValidCard$ Card | Activator$ You | ValidSpell$ Activated.!ManaAbility | Amount$ 2 | MinMana$ 1",
        ),
        (
            "unknown activated keyword reduction",
            "Name:CrewDisc\nManaCost:1 R\nTypes:Creature\nPT:2/2\n\
             S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Crew | Activator$ You | Amount$ 1",
        ),
        (
            "colored mana reduction on activated ability",
            "Name:ColorAb\nManaCost:2 U\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Creature.YouCtrl | Type$ Ability | Color$ U | Amount$ 1 | AffectedZone$ Battlefield",
        ),
        (
            "ValidTarget with Exhaust reduction",
            "Name:ExhaustTgt\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
             S:Mode$ ReduceCost | ValidCard$ Permanent.YouCtrl | ValidSpell$ Activated.Exhaust | ValidTarget$ Card.Self | Amount$ 2 | AffectedZone$ Battlefield",
        ),
        (
            "unhandled CheckSVar on ability reduction",
            "Name:FirstAb\nManaCost:2 U\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Artifact.YouCtrl | Type$ Ability | Amount$ X | CheckSVar$ N | SVarCompare$ LE1 | AffectedZone$ Battlefield\n\
             SVar:X:Count$CardPower\n\
             SVar:N:Count$ThisTurnActivated_Activated.YouCtrl",
        ),
        (
            "unscoped Card without AffectedZone",
            "Name:UnscopedExhaust\nManaCost:2 G\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Exhaust | Activator$ You | Amount$ 2",
        ),
        (
            "zero exhaust reduction",
            "Name:ZeroExhaust\nManaCost:2 G\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Permanent.YouCtrl | ValidSpell$ Activated.Exhaust | Amount$ 0 | AffectedZone$ Battlefield",
        ),
        (
            "RememberObjects on ability reduction",
            "Name:RemAb\nManaCost:2 G\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Creature.YouCtrl | Type$ Ability | Amount$ X | RememberObjects$ Self | AffectedZone$ Battlefield\n\
             SVar:X:Count$CardPower",
        ),
    ] {
        assert!(refused_w27d(script), "expected refusal: {label}");
    }
}

// =========================================================================
// 2. Modifier::FirstSpellEachTurnCostAdjust (Artificer Class, Acolyte, etc.)
// =========================================================================

#[test]
fn first_spell_each_turn_cost_adjust_positive_shapes() {
    // Artificer Class: first artifact spell you cast each turn costs {1} less.
    let artificer = read_w27d(
        "Name:Artificer Class\nManaCost:1 U\nTypes:Enchantment Class\n\
         S:Mode$ ReduceCost | EffectZone$ Battlefield | ValidCard$ Card.Artifact | Activator$ You | Type$ Spell | OnlyFirstSpell$ True | Amount$ 1",
    );
    assert_eq!(
        artificer.abilities[0],
        "static_ability!(Filter::Any, Modifier::FirstSpellEachTurnCostAdjust { \
         who: PlayerRel::You, filter: &Filter::ARTIFACT, cost: mana!(\"{1}\"), \
         increase: false, colored_only: false })"
    );

    // Acolyte of Bahamut: Background granting FirstSpellEachTurnCostAdjust to commander.
    let acolyte = read_w27d(
        "Name:Acolyte of Bahamut\nManaCost:1 G\nTypes:Legendary Enchantment Background\n\
         S:Mode$ Continuous | Affected$ Creature.IsCommander+YouOwn | AddStaticAbility$ DragonReduce\n\
         SVar:DragonReduce:Mode$ ReduceCost | EffectZone$ Battlefield | ValidCard$ Card.Dragon | Activator$ You | Type$ Spell | OnlyFirstSpell$ True | Amount$ 2",
    );
    assert!(
        acolyte.abilities[0].contains("Modifier::GrantStatic")
            && acolyte.abilities[0].contains("Modifier::FirstSpellEachTurnCostAdjust")
            && acolyte.abilities[0].contains("who: PlayerRel::You")
            && acolyte.abilities[0].contains("cost: mana!(\"{2}\")")
            && acolyte.abilities[0].contains("increase: false"),
        "{:?}",
        acolyte.abilities
    );

    // Kadena, Slinking Sorcerer: first face-down creature spell costs {3} less.
    let kadena = read_w27d(
        "Name:Kadena\nManaCost:1 B G U\nTypes:Legendary Creature\nPT:3/3\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.faceDown | Type$ Spell | OnlyFirstSpell$ True | Activator$ You | Amount$ 3",
    );
    assert!(
        kadena.abilities[0].contains("Modifier::FirstSpellEachTurnCostAdjust")
            && kadena.abilities[0].contains("cost: mana!(\"{3}\")"),
        "{:?}",
        kadena.abilities
    );
    assert!(
        kadena.statics.contains("Filter::FaceDown") && kadena.statics.contains("Filter::CREATURE"),
        "{:?}",
        kadena.statics
    );

    // Melek, Reforged Researcher: ValidSpell$ Spell.Instant,Spell.Sorcery without InZone(Stack).
    let melek = read_w27d(
        "Name:Melek\nManaCost:3 U R\nTypes:Legendary Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | OnlyFirstSpell$ True | Type$ Spell | ValidSpell$ Spell.Instant,Spell.Sorcery | Activator$ You | Amount$ 3",
    );
    assert!(
        melek.abilities[0].contains("Modifier::FirstSpellEachTurnCostAdjust")
            && melek.abilities[0].contains("cost: mana!(\"{3}\")"),
        "{:?}",
        melek.abilities
    );
    assert!(
        !melek.abilities[0].contains("ZoneRef::Stack") && !melek.statics.contains("ZoneRef::Stack"),
        "spell filter must not require Stack zone: {:?} {:?}",
        melek.abilities,
        melek.statics
    );

    // Vine Gecko: ValidSpell$ Spell.Kicked -> Filter::Kicked.
    let vine = read_w27d(
        "Name:Vine Gecko\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | OnlyFirstSpell$ True | Type$ Spell | ValidSpell$ Spell.Kicked | Activator$ You | Amount$ 1",
    );
    assert_eq!(
        vine.abilities[0],
        "static_ability!(Filter::Any, Modifier::FirstSpellEachTurnCostAdjust { \
         who: PlayerRel::You, filter: &Filter::Kicked, cost: mana!(\"{1}\"), \
         increase: false, colored_only: false })"
    );

    // Colored reduction with IgnoreGeneric$ True (colored_only: true) and Condition$ PlayerTurn.
    let colored = read_w27d(
        "Name:ColorFirst\nManaCost:W B\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Creature | Type$ Spell | OnlyFirstSpell$ True | Activator$ You | Condition$ PlayerTurn | Color$ W B | Amount$ 1 | IgnoreGeneric$ True",
    );
    assert!(
        colored.abilities[0].contains("Modifier::FirstSpellEachTurnCostAdjust")
            && colored.abilities[0].contains("cost: mana!(\"{W}{B}\")")
            && colored.abilities[0].contains("increase: false")
            && colored.abilities[0].contains("colored_only: true")
            && colored.abilities[0].contains("condition = Some(Condition::YourTurn)"),
        "{:?}",
        colored.abilities
    );

    // RaiseCost with OnlyFirstSpell$ True (generic + colored mana + Command zone).
    let tax = read_w27d(
        "Name:FirstTax\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | EffectZone$ Battlefield | ValidCard$ Card.nonCreature | Activator$ Player.Opponent | Type$ Spell | OnlyFirstSpell$ True | Amount$ 1\n\
         S:Mode$ RaiseCost | EffectZone$ Battlefield | ValidCard$ Card.Creature | Activator$ Player | Type$ Spell | OnlyFirstSpell$ True | Cost$ W",
    );
    assert_eq!(
        tax.abilities[0],
        "static_ability!(Filter::Any, Modifier::FirstSpellEachTurnCostAdjust { \
         who: PlayerRel::EachOpponent, filter: &Filter::NONCREATURE, cost: mana!(\"{1}\"), \
         increase: true, colored_only: false })"
    );
    assert_eq!(
        tax.abilities[1],
        "static_ability!(Filter::Any, Modifier::FirstSpellEachTurnCostAdjust { \
         who: PlayerRel::EachPlayer, filter: &Filter::CREATURE, cost: mana!(\"{W}\"), \
         increase: true, colored_only: false })"
    );
}

#[test]
fn first_spell_each_turn_cost_adjust_honest_refusals() {
    for (label, script) in [
        (
            "Head of the Class: targeting condition in ValidSpell$",
            "Name:HeadOfClass\nManaCost:W B\nTypes:Creature\nPT:2/2\n\
             S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | ValidSpell$ Spell.IsTargeting Valid Creature | Activator$ You | OnlyFirstSpell$ True | Amount$ 1 | Color$ W B | IgnoreGeneric$ True",
        ),
        (
            "Eluge: dynamic Amount$ Y with OnlyFirstSpell$",
            "Name:Eluge\nManaCost:1 U U U\nTypes:Legendary Creature\nPT:4/4\n\
             S:Mode$ ReduceCost | EffectZone$ Battlefield | ValidCard$ Instant,Sorcery | Activator$ You | Type$ Spell | OnlyFirstSpell$ True | Color$ U | Amount$ Y\n\
             SVar:Y:Count$Valid Land.YouCtrl",
        ),
        (
            "OnlyFirstSpell with ValidTarget$",
            "Name:FirstTgt\nManaCost:2 U\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | OnlyFirstSpell$ True | ValidTarget$ Creature.YouCtrl | Activator$ You | Amount$ 1",
        ),
        (
            "OnlyFirstSpell on Card.Self",
            "Name:FirstSelf\nManaCost:3 U\nTypes:Instant\n\
             S:Mode$ ReduceCost | ValidCard$ Card.Self | EffectZone$ All | Type$ Spell | OnlyFirstSpell$ True | Activator$ You | Amount$ 1\n\
             A:SP$ Draw | NumCards$ 1",
        ),
        (
            "OnlyFirstSpell on Type$ Ability",
            "Name:FirstAb\nManaCost:2 U\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Artifact.YouCtrl | Type$ Ability | OnlyFirstSpell$ True | Activator$ You | Amount$ 2",
        ),
        (
            "OnlyFirstSpell with Relative$ True",
            "Name:FirstRel\nManaCost:2 U\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | OnlyFirstSpell$ True | Relative$ True | Activator$ You | Amount$ 1",
        ),
        (
            "OnlyFirstSpell RaiseCost with Color$",
            "Name:FirstRaiseColor\nManaCost:2 B\nTypes:Enchantment\n\
             S:Mode$ RaiseCost | ValidCard$ Card | Type$ Spell | OnlyFirstSpell$ True | Activator$ Opponent | Color$ B | Amount$ 1",
        ),
        (
            "OnlyFirstSpell with IgnoreGeneric$ without Color$",
            "Name:FirstBadIgnore\nManaCost:2 U\nTypes:Enchantment\n\
             S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | OnlyFirstSpell$ True | Activator$ You | Amount$ 2 | IgnoreGeneric$ True",
        ),
    ] {
        assert!(refused_w27d(script), "expected refusal: {label}");
    }
}

// =========================================================================
// 3. Modifier::MustAttackPlayer (Alluring Siren, Public Enemy, Kardur, etc.)
// =========================================================================

#[test]
fn must_attack_player_static_and_temporary_effects() {
    // Alluring Siren: activated AB$ Effect forcing target creature an opponent controls to attack you.
    let siren = read_w27d(
        "Name:Alluring Siren\nManaCost:1 U\nTypes:Creature Siren\nPT:1/1\n\
         A:AB$ Effect | Cost$ T | ValidTgts$ Creature.OppCtrl | TgtPrompt$ Select target creature an opponent controls | RememberObjects$ Targeted | ExileOnMoved$ Battlefield | StaticAbilities$ MustAttack | IsCurse$ True\n\
         SVar:MustAttack:Mode$ MustAttack | ValidCreature$ Creature.IsRemembered | MustAttack$ You",
    );
    assert!(
        siren.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::MustAttackPlayer(PlayerRel::You), duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        siren.abilities
    );

    // Public Enemy: static Aura forcing all creatures to attack enchanted creature's controller.
    let public_enemy = read_w27d(
        "Name:Public Enemy\nManaCost:2 U\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ MustAttack | ValidCreature$ Creature | MustAttack$ EnchantedController\n\
         T:Mode$ ChangesZone | ValidCard$ Card.AttachedBy | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert!(
        public_enemy.abilities.iter().any(|a| {
            a == "static_ability!(Filter::CREATURE, Modifier::MustAttackPlayer(PlayerRel::ControllerOfAttached))"
        }),
        "{:?}",
        public_enemy.abilities
    );

    // Kardur, Doomscourge: DB$ Effect with two MustAttack statics (AttacksEachCombat + Player.Other).
    let kardur = read_w27d(
        "Name:Kardur, Doomscourge\nManaCost:2 B R\nTypes:Legendary Creature\nPT:4/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEffect\n\
         SVar:TrigEffect:DB$ Effect | StaticAbilities$ AttackEach,AttackOther | Duration$ UntilYourNextTurn\n\
         SVar:AttackEach:Mode$ MustAttack | ValidCreature$ Creature.OppCtrl\n\
         SVar:AttackOther:Mode$ MustAttack | ValidCreature$ Creature.OppCtrl | MustAttack$ Player.Other | Secondary$ True",
    );
    assert!(
        kardur.abilities[0].contains("Modifier::AttacksEachCombat")
            && kardur.abilities[0]
                .contains("Modifier::MustAttackPlayer(PlayerRel::EachOtherPlayer)")
            && kardur.abilities[0].contains("Duration::UntilYourNextTurn"),
        "{:?}",
        kardur.abilities
    );

    // Nahiri [+1] shape: MustAttack$ Player -> PlayerRel::EachPlayer.
    let nahiri_plus = read_w27d(
        "Name:ForcePlayerAttack\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | ExileOnMoved$ Battlefield | Duration$ UntilYourNextTurn | StaticAbilities$ MustAttack\n\
         SVar:MustAttack:Mode$ MustAttack | MustAttack$ Player | ValidCreature$ Card.IsRemembered",
    );
    assert!(
        nahiri_plus.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::MustAttackPlayer(PlayerRel::EachPlayer), duration: Duration::UntilYourNextTurn }"
        ),
        "{:?}",
        nahiri_plus.abilities
    );

    // ChosenPlayer with UntilEndOfTurn on a permanent with ChooseOpponent lowers via AddTempModifier.
    let chosen_eot = read_w27d(
        "Name:ChosenAttackEot\nManaCost:2 R\nTypes:Creature\nPT:3/3\n\
         K:ETBReplacement:Other:ChooseOpp\n\
         SVar:ChooseOpp:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent\n\
         A:AB$ Effect | Cost$ T | StaticAbilities$ AttackChosen\n\
         SVar:AttackChosen:Mode$ MustAttack | ValidCreature$ Card.EffectSource | MustAttack$ ChosenPlayer",
    );
    assert!(
        chosen_eot.abilities[0].contains(
            "Effect::AddTempModifier { target: TargetSpec::ThisObject, modifier: Modifier::MustAttackPlayer(PlayerRel::Chosen), duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        chosen_eot.abilities
    );
}

#[test]
fn must_attack_player_honest_refusals() {
    for (label, script) in [
        (
            "MustAttack$ EffectSource (planeswalker target, not a player)",
            "Name:GideonLike\nManaCost:3 W W\nTypes:Legendary Planeswalker\nLoyalty:6\n\
             A:AB$ Effect | Cost$ AddCounter<2/LOYALTY> | Planeswalker$ True | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ MA\n\
             SVar:MA:Mode$ MustAttack | ValidCreature$ Creature.IsRemembered | MustAttack$ EffectSource",
        ),
        (
            "MustAttack$ ChosenPlayer with UntilYourNextUpkeep (unsupported duration)",
            "Name:RavingLike\nManaCost:4 B\nTypes:Creature\nPT:2/6\n\
             T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigChoose\n\
             SVar:TrigChoose:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | Random$ True | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | StaticAbilities$ AttackChosen | Duration$ UntilYourNextUpkeep\n\
             SVar:AttackChosen:Mode$ MustAttack | ValidCreature$ Card.EffectSource | MustAttack$ ChosenPlayer",
        ),
        (
            "MustAttack$ Opponent.lifeEQX (life comparison expression)",
            "Name:GalactusLike\nManaCost:6\nTypes:Creature\nPT:10/10\n\
             S:Mode$ MustAttack | ValidCreature$ Card.Self | MustAttack$ Opponent.lifeEQX",
        ),
        (
            "MustAttack$ RememberedPlayer",
            "Name:RemPlayer\nManaCost:2 R\nTypes:Sorcery\n\
             A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ MA\n\
             SVar:MA:Mode$ MustAttack | ValidCreature$ Card.IsRemembered | MustAttack$ RememberedPlayer",
        ),
    ] {
        assert!(refused_w27d(script), "expected refusal: {label}");
    }
}

// =========================================================================
// 4. Filter::Saddled (IsSaddled / !IsSaddled)
// =========================================================================

#[test]
fn saddled_filter_atom_triggers_statics_and_targets() {
    // Lagorin, Soul of Alacria: attacks while saddled trigger.
    let lagorin = read_w27d(
        "Name:Lagorin, Soul of Alacria\nManaCost:G W\nTypes:Legendary Creature Beast Mount\nPT:1/1\n\
         K:Flying\n\
         T:Mode$ Attacks | ValidCard$ Card.Self+IsSaddled | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Mount,Vehicle | CounterType$ P1P1 | CounterNum$ 1\n\
         K:Saddle:1",
    );
    assert!(
        lagorin.statics.contains("Filter::This") && lagorin.statics.contains("Filter::Saddled"),
        "{:?}",
        lagorin.statics
    );

    // Gloryheath Lynx: attacks while saddled search.
    let lynx = read_w27d(
        "Name:Gloryheath Lynx\nManaCost:1 W\nTypes:Creature Cat Mount\nPT:2/3\n\
         K:Lifelink\n\
         T:Mode$ Attacks | ValidCard$ Card.Self+IsSaddled | TriggerZones$ Battlefield | Execute$ TrigSearch\n\
         SVar:TrigSearch:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Plains.Basic\n\
         K:Saddle:2",
    );
    assert!(
        lynx.statics.contains("Filter::This") && lynx.statics.contains("Filter::Saddled"),
        "{:?}",
        lynx.statics
    );

    // Static anthem + targeting + negation atoms.
    let buff_and_destroy = read_w27d(
        "Name:SaddleCheck\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.IsSaddled+YouCtrl | AddPower$ 2 | AddToughness$ 2\n\
         A:AB$ Destroy | Cost$ 2 W | ValidTgts$ Creature.!IsSaddled",
    );
    assert!(
        buff_and_destroy.abilities[0].contains("Filter::Saddled"),
        "{:?}",
        buff_and_destroy.abilities
    );
    assert!(
        buff_and_destroy
            .statics
            .contains("Filter::Not(&Filter::Saddled)"),
        "{:?}",
        buff_and_destroy.statics
    );
}

#[test]
fn saddled_honest_refusals() {
    for (label, script) in [
        (
            "SaddledThisTurn is a distinct predicate (creatures that saddled this Mount)",
            "Name:GiantBeaverLike\nManaCost:3 G\nTypes:Creature Beast Mount\nPT:4/4\n\
             T:Mode$ Attacks | ValidCard$ Card.Self+IsSaddled | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
             SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature.SaddledThisTurn | CounterType$ P1P1\n\
             K:Saddle:3",
        ),
        (
            "T:Mode$ Saddled trigger mode stays refused",
            "Name:CanyonVaulterLike\nManaCost:1 W\nTypes:Creature\nPT:2/2\n\
             T:Mode$ Saddled | ValidCrew$ Card.Self | Execute$ TrigDraw | TriggerZones$ Battlefield\n\
             SVar:TrigDraw:DB$ Draw",
        ),
        (
            "Alchemy Seek inside attacks-while-saddled trigger stays refused",
            "Name:RoutewayMooseLike\nManaCost:2 G\nTypes:Creature Mount\nPT:3/3\n\
             T:Mode$ Attacks | ValidCard$ Card.Self+IsSaddled | TriggerZones$ Battlefield | Execute$ TrigSeek\n\
             SVar:TrigSeek:DB$ Seek | Type$ Land\n\
             K:Saddle:2",
        ),
    ] {
        assert!(refused_w27d(script), "expected refusal: {label}");
    }
}

// =========================================================================
// 5. Owner Exile Play Permission (Aerial Extortionist, Release to the Wind, Prowl)
// =========================================================================

#[test]
fn owner_exile_play_permission_aerial_extortionist_release_and_prowl() {
    // Aerial Extortionist: ChangeZone (no RememberChanged$) + DB$ Effect | EffectOwner$ TargetedOwner.
    let extortionist = read_w27d(
        "Name:Aerial Extortionist\nManaCost:3 W W\nTypes:Creature Bird Soldier\nPT:4/3\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigExile | TriggerZones$ Battlefield | Secondary$ True\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Permanent.nonLand | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Targeted | StaticAbilities$ Play | ForgetOnMoved$ Exile | EffectOwner$ TargetedOwner | Duration$ Permanent\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile | Description$ For as long as that card remains exiled, its owner may cast it.\n\
         T:Mode$ SpellCast | ValidCard$ Card.!wasCastFromTheirHand | ValidActivatingPlayer$ Player.Other | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert!(
        extortionist.abilities.iter().any(|a| {
            a.contains("Effect::ExileWithPermission")
                && a.contains(
                    "ExilePermission::cast(PlayWindow::WhileExiled).for_player(PlayerRel::OwnerOfTarget)"
                )
        }),
        "{:?}",
        extortionist.abilities
    );

    // Release to the Wind: RememberObjects$ ParentTarget + MayPlayWithoutManaCost$ True + DBCleanup.
    let release = read_w27d(
        "Name:Release to the Wind\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent.nonLand | Origin$ Battlefield | SubAbility$ DBEffect | Destination$ Exile\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ ParentTarget | EffectOwner$ TargetedOwner | StaticAbilities$ STPlay | Duration$ Permanent | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | MayPlayWithoutManaCost$ True | Affected$ Card.IsRemembered+nonLand | AffectedZone$ Exile | Description$ For as long as that card remains exiled, its owner may cast it without paying its mana cost.\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert_eq!(
        release.abilities[0],
        "spell!(&[Effect::ExileWithPermission { target: TargetSpec::Object(&Filter::NONLAND), \
         until: None, permission: ExilePermission::cast(PlayWindow::WhileExiled)\
         .without_paying_mana_cost().for_player(PlayerRel::OwnerOfTarget) }], \
         targets = Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND))))"
    );

    // Prowl front-face shape: "its owner may play it" -> ExilePermission::play(...)
    let prowl = read_w27d(
        "Name:ProwlLike\nManaCost:3 W\nTypes:Legendary Artifact Creature Robot\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.Other+tapped,Vehicle.Other+tapped | TargetMin$ 0 | TargetMax$ 1 | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Targeted | StaticAbilities$ Play | ForgetOnMoved$ Exile | EffectOwner$ TargetedOwner | Duration$ Permanent\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile | Description$ For as long as that card remains exiled, its owner may play it.",
    );
    assert!(
        prowl.abilities[0].contains(
            "ExilePermission::play(PlayWindow::WhileExiled).for_player(PlayerRel::OwnerOfTarget)"
        ),
        "{:?}",
        prowl.abilities
    );

    // Dig top-of-library exile with EffectOwner$ TriggeredSourceController on DamageDone (Curse of Hospitality shape).
    let hospitality = read_w27d(
        "Name:HospitalityLike\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ DamageDone | ValidSource$ Creature | ValidTarget$ Player | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ Dig | Defined$ TriggeredTarget | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | EffectOwner$ TriggeredSourceController | Duration$ EndOfTurn | RememberObjects$ Remembered | StaticAbilities$ STPlay | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreColor$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        hospitality.abilities[0].contains(
            "Effect::ExileTopWithPermission { who: PlayerRel::DamagedPlayer, count: Amount::Fixed(1), \
             permission: ExilePermission::play(PlayWindow::ThisTurn).spend_mana_as_any_color().for_player(PlayerRel::ControllerOfEvent) }"
        ),
        "{:?}",
        hospitality.abilities
    );
}

#[test]
fn owner_exile_play_permission_honest_refusals() {
    for (label, script) in [
        (
            "RememberObjects$ You & Targeted with Triggers$ (Palace Jailer shape)",
            "Name:JailerLike\nManaCost:2 W W\nTypes:Creature\nPT:2/2\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
             SVar:TrigExile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature.OppCtrl | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | Triggers$ ComeBack | RememberObjects$ You & Targeted | Duration$ Permanent | EffectOwner$ TargetedOwner | ForgetOnMoved$ Exile\n\
             SVar:ComeBack:Mode$ Phase | Phase$ Upkeep | Execute$ DBDraw\n\
             SVar:DBDraw:DB$ Draw",
        ),
        (
            "Duration$ UntilYourNextEndStep (Casey & Raph: not a supported PlayWindow)",
            "Name:CaseyLike\nManaCost:4 R\nTypes:Instant\n\
             A:SP$ Dig | ValidTgts$ Player | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | StaticAbilities$ CRPlay | EffectOwner$ RememberedOwner | Duration$ UntilYourNextEndStep | ExileOnMoved$ Exile | RememberObjects$ Remembered | SubAbility$ DBCleanup\n\
             SVar:CRPlay:Mode$ Continuous | Affected$ Card.IsRemembered | MayPlay$ True | AffectedZone$ Exile\n\
             SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        ),
        (
            "EffectOwner$ RememberedController on battlefield ChangeZone (controller != owner)",
            "Name:StolenCtrl\nManaCost:2 W\nTypes:Instant\n\
             A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ Play | ForgetOnMoved$ Exile | EffectOwner$ RememberedController | Duration$ Permanent\n\
             SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile",
        ),
        (
            "EffectOwner$ TargetedOwner without target (Self exile)",
            "Name:SelfTargetedOwner\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
             A:AB$ ChangeZone | Cost$ T | Defined$ Self | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ Play | ForgetOnMoved$ Exile | EffectOwner$ TargetedOwner | Duration$ Permanent\n\
             SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile",
        ),
        (
            "EffectOwner$ RememberedOwner with extra MayLookAt$ parameter on Play static",
            "Name:FaceDownOwner\nManaCost:2 B\nTypes:Instant\n\
             A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | RememberObjects$ Targeted | StaticAbilities$ Play | ForgetOnMoved$ Exile | EffectOwner$ TargetedOwner | Duration$ Permanent\n\
             SVar:Play:Mode$ Continuous | MayLookAt$ You | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile",
        ),
        (
            "EffectOwner$ TriggeredSourceController outside a DamageDone trigger",
            "Name:NonDamageTrigOwner\nManaCost:2 R\nTypes:Enchantment\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigExile\n\
             SVar:TrigExile:DB$ Dig | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | EffectOwner$ TriggeredSourceController | Duration$ EndOfTurn | RememberObjects$ Remembered | StaticAbilities$ STPlay | ForgetOnMoved$ Exile\n\
             SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile",
        ),
    ] {
        assert!(refused_w27d(script), "expected refusal: {label}");
    }
}
