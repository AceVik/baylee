//! Wave 16c scriptgen tests: atomic delayed observers (`DB$ Effect`),
//! `R:Event$ Moved` & `K:ETBReplacement:Other:` entry replacements/choices,
//! `ChoosePlayer` resolution chains, and `ChooseNumber` variable payments.

use super::*;

fn wave16c_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Aetherborn".into(),
            "Elf".into(),
            "Goblin".into(),
            "Soldier".into(),
            "Spirit".into(),
            "Wall".into(),
            "Wizard".into(),
        ],
        artifact: vec!["Clue".into(), "Treasure".into()],
        enchantment: vec!["Aura".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn wave16c_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "w_1_1_soldier",
            "Name:Soldier\nTypes:Creature Soldier\nColors:white\nPT:1/1",
        ),
        (
            "w_1_1_spirit_flying",
            "Name:Spirit\nTypes:Creature Spirit\nColors:white\nPT:1/1\nK:Flying",
        ),
        (
            "b_x_x_aetherborn",
            "Name:Aetherborn\nTypes:Creature Aetherborn\nColors:black\nPT:*/*",
        ),
    ])
}

fn lower_w16c(text: &str) -> CardBody {
    let cats = wave16c_cats();
    let toks = wave16c_tokens();
    let script = parse(text);
    transcode(&script, &cats, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "expected transcode to succeed: {:?}",
            refusal_reason(&script, &cats, Some(&toks))
        )
    })
}

fn refused_w16c(text: &str) -> bool {
    let cats = wave16c_cats();
    let toks = wave16c_tokens();
    transcode(&parse(text), &cats, Some(&toks)).is_none()
}

#[test]
fn arcbond_auriok_siege_sled_basri_skullcrack_and_delayed_effect_triggers() {
    // 1. Arcbond: watched DamageDoneOnce -> EventObjectDealsDamageEach.
    let arcbond = lower_w16c(
        "Name:ArcbondProbe\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | Triggers$ TrigDealtDmg | ExileOnMoved$ Battlefield | RememberObjects$ Targeted\n\
         SVar:TrigDealtDmg:Mode$ DamageDoneOnce | ValidTarget$ Card.IsRemembered | Execute$ DmgOther\n\
         SVar:DmgOther:DB$ DamageAll | ValidCards$ Creature.!IsRemembered | ValidPlayers$ Player | NumDmg$ X | DamageSource$ Remembered\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    assert!(
        arcbond.abilities[0].contains("Effect::EventObjectDealsDamageEach")
            && arcbond.abilities[0].contains("exclude_event_object: true"),
        "{:?}",
        arcbond.abilities
    );

    // 2. Auriok Siege Sled: UntilHostLeavesPlayOrEOT + CantBlock(&Filter::This).
    let sled = lower_w16c(
        "Name:SiegeSledProbe\nManaCost:6\nTypes:Artifact Creature\nPT:3/5\n\
         A:AB$ Effect | Cost$ 1 | ValidTgts$ Creature.Artifact | IsCurse$ True | RememberObjects$ ThisTargetedCard | ForgetOnMoved$ Battlefield | StaticAbilities$ KWPump | Duration$ UntilHostLeavesPlayOrEOT\n\
         SVar:KWPump:Mode$ CantBlockBy | ValidAttacker$ Creature.EffectSource | ValidBlocker$ Creature.IsRemembered",
    );
    assert!(
        sled.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::CantBlock(&Filter::This), duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        sled.abilities
    );

    // 3. Basri Ket [-2]: AttackersDeclared + TriggerObjectsAttackers$Valid -> EventAmount.
    let basri = lower_w16c(
        "Name:BasriProbe\nManaCost:1 W W\nTypes:Legendary Planeswalker\nLoyalty:3\n\
         A:AB$ Effect | Cost$ SubCounter<2/LOYALTY> | Planeswalker$ True | Triggers$ TrigAttack\n\
         SVar:TrigAttack:Mode$ AttackersDeclared | ValidAttackers$ Creature.!token | TriggerZones$ Command | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenAmount$ BasriX | TokenScript$ w_1_1_soldier | TokenOwner$ You | TokenTapped$ True | TokenAttacking$ True\n\
         SVar:BasriX:TriggerObjectsAttackers$Valid Creature.!token",
    );
    assert!(
        basri.abilities[0].contains("Trigger::OneOrMoreAttack(")
            && basri.abilities[0].contains("Effect::CreateTokenAttacking")
            && basri.abilities[0].contains("amount: Amount::EventAmount"),
        "{:?}",
        basri.abilities
    );

    // 4. Call In a Professional / Leyline of Punishment: DamageCantBePrevented.
    let call = lower_w16c(
        "Name:NoPreventSpell\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ STCantGain,STCantPrevent | SubAbility$ DBDamage\n\
         SVar:STCantGain:Mode$ CantGainLife | ValidPlayer$ Player\n\
         SVar:STCantPrevent:Mode$ CantPreventDamage\n\
         SVar:DBDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | NoPrevention$ True",
    );
    assert!(
        call.abilities[0].contains("Modifier::DamageCantBePrevented")
            && call.abilities[0].contains("Modifier::PlayersCantGainLife(PlayerRel::EachPlayer)"),
        "{:?}",
        call.abilities
    );

    let leyline = lower_w16c(
        "Name:NoPreventEnchant\nManaCost:2 R R\nTypes:Enchantment\n\
         S:Mode$ CantPreventDamage",
    );
    assert_eq!(
        leyline.abilities,
        ["static_ability!(Filter::Any, Modifier::DamageCantBePrevented)"]
    );

    // 5. Don't Move & Electric Seaweed & Ember Gale.
    let dont_move = lower_w16c(
        "Name:DontMoveProbe\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Creature.tapped | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | Triggers$ TrigTaps | Duration$ UntilYourNextTurn\n\
         SVar:TrigTaps:Mode$ Taps | ValidCard$ Creature | Execute$ TrigDestroy\n\
         SVar:TrigDestroy:DB$ Destroy | Defined$ TriggeredCardLKICopy",
    );
    assert!(
        dont_move.abilities[0].contains("Trigger::BecomesTapped(")
            && dont_move.abilities[0].contains("Effect::destroy(TargetSpec::EventObject)"),
        "{:?}",
        dont_move.abilities
    );

    let seaweed = lower_w16c(
        "Name:SeaweedProbe\nManaCost:2 R R\nTypes:Creature\nPT:0/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEffect\n\
         SVar:TrigEffect:DB$ Effect | Triggers$ DieTrigger | RememberObjects$ Self\n\
         SVar:DieTrigger:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.!IsRemembered | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DamageAll | ValidCards$ Creature.nonWall | NumDmg$ 1 | DamageSource$ EffectSource",
    );
    assert!(
        seaweed.abilities[0].contains("watch: TriggerRemember::Source")
            && seaweed.abilities[0].contains("Effect::DealDamageEach"),
        "{:?}",
        seaweed.abilities
    );

    let ember_gale = lower_w16c(
        "Name:EmberGaleProbe\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Player | StaticAbilities$ OppCantBlock | RememberObjects$ Targeted\n\
         SVar:OppCantBlock:Mode$ CantBlock | ValidCard$ Creature.RememberedPlayerCtrl",
    );
    assert!(
        ember_gale.abilities[0].contains("Effect::AddGlobalTempModifier")
            && ember_gale
                .statics
                .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{:?} / {}",
        ember_gale.abilities,
        ember_gale.statics
    );
}

#[test]
fn archelos_banner_canker_daytime_and_etb_player_or_number_choices() {
    // 1. Archelos, Lagoon Mystic: EnterTappedIf + EnterUntappedIf.
    let archelos = lower_w16c(
        "Name:LagoonTurtle\nManaCost:1 B G U\nTypes:Legendary Creature\nPT:2/4\n\
         R:Event$ Moved | ValidCard$ Permanent | Destination$ Battlefield | IsPresent$ Card.Self+tapped | ReplaceWith$ ETBTapped | ReplacementResult$ Updated | ActiveZones$ Battlefield\n\
         SVar:ETBTapped:DB$ Tap | ETB$ True | Defined$ ReplacedCard\n\
         R:Event$ Moved | ValidCard$ Permanent | Destination$ Battlefield | IsPresent$ Card.Self+untapped | ReplaceWith$ ETBUntapped | ReplacementResult$ Updated | ActiveZones$ Battlefield\n\
         SVar:ETBUntapped:DB$ Untap | ETB$ True | Defined$ ReplacedCard",
    );
    assert_eq!(
        archelos.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::EnterTappedIf { \
             filter: &Filter::Any, condition: Condition::SourceMatches(&Filter::Tapped) })",
            "AbilityDef::Replacement(ReplacementRule::EnterUntappedIf { \
             filter: &Filter::Any, condition: Condition::SourceMatches(&Filter::Untapped) })",
        ]
    );

    // 2. Banner of Kinship & Canker Abomination: pre-counter entry choice + WithCounters.
    let banner = lower_w16c(
        "Name:KinshipBanner\nManaCost:5\nTypes:Artifact\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ChooseCT | ReplacementResult$ Updated\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ DBCounters\n\
         SVar:DBCounters:DB$ PutCounter | ETB$ True | Defined$ Self | CounterType$ FELLOWSHIP | CounterNum$ X\n\
         SVar:X:Count$Valid Creature.ChosenType+YouCtrl",
    );
    assert_eq!(banner.enter_modifiers[0], "EnterModifier::ChooseSubtype");
    assert!(banner.enter_modifiers[1].starts_with("EnterModifier::WithCounters {"));

    let canker = lower_w16c(
        "Name:CankerProbe\nManaCost:2 B G\nTypes:Creature\nPT:6/6\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ChooseP | ReplacementResult$ Updated\n\
         SVar:ChooseP:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | SubAbility$ DBCounters\n\
         SVar:DBCounters:DB$ PutCounter | ETB$ True | Defined$ Self | CounterType$ M1M1 | CounterNum$ X\n\
         SVar:X:Count$Valid Creature.ChosenCtrl",
    );
    assert_eq!(canker.enter_modifiers[0], "EnterModifier::ChooseOpponent");
    assert!(canker.enter_modifiers[1].starts_with("EnterModifier::WithCounters {"));
    assert!(
        canker
            .statics
            .contains("Filter::ControlledBy(PlayerRel::ChosenBySource)")
    );

    // 3. Brimstone Vandal: BecomeDayIfNeither + Trigger::DayNightChanges.
    let vandal = lower_w16c(
        "Name:DayVandal\nManaCost:2 R\nTypes:Creature\nPT:2/3\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | DayTime$ Neither | ReplaceWith$ DoDay | ReplacementResult$ Updated\n\
         SVar:DoDay:DB$ DayTime | Value$ Day\n\
         T:Mode$ DayTimeChanges | Execute$ TrigDamage | TriggerZones$ Battlefield\n\
         SVar:TrigDamage:DB$ DealDamage | Defined$ Player.Opponent | NumDmg$ 1",
    );
    assert_eq!(
        vandal.enter_modifiers,
        ["EnterModifier::BecomeDayIfNeither"]
    );
    assert!(vandal.abilities[0].starts_with("triggered!(Trigger::DayNightChanges,"));

    // 4. True-Name Nemesis, Sorcerous Spyglass, Talion, Sanctum Prelate.
    let tnn = lower_w16c(
        "Name:TrueNameProbe\nManaCost:1 U U\nTypes:Creature\nPT:3/1\n\
         K:ETBReplacement:Other:ChooseP\n\
         SVar:ChooseP:DB$ ChoosePlayer | Defined$ You | Choices$ Player\n\
         S:Mode$ Continuous | AddKeyword$ Protection:Player.PlayerUID_ChosenPlayerUID:ChosenPlayerName | AffectedDefined$ Self",
    );
    assert_eq!(tnn.enter_modifiers, ["EnterModifier::ChooseAnyPlayer"]);
    assert!(
        tnn.abilities[0]
            .contains("Modifier::ProtectionFrom(&Filter::ControlledBy(PlayerRel::ChosenBySource))"),
        "{:?}",
        tnn.abilities
    );

    let spyglass = lower_w16c(
        "Name:SpyglassProbe\nManaCost:2\nTypes:Artifact\n\
         K:ETBReplacement:Other:ChoosePlayer\n\
         SVar:ChoosePlayer:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | SubAbility$ DBLook\n\
         SVar:DBLook:DB$ RevealHand | Defined$ ChosenPlayer | Look$ True | SubAbility$ DBNameCard\n\
         SVar:DBNameCard:DB$ NameCard | Defined$ You | SubAbility$ DBClear\n\
         SVar:DBClear:DB$ Cleanup | ClearChosenPlayer$ True\n\
         S:Mode$ CantBeActivated | ValidCard$ Card.NamedCard | ValidSA$ Activated.!ManaAbility",
    );
    assert_eq!(
        spyglass.enter_modifiers,
        [
            "EnterModifier::LookAtOpponentHand",
            "EnterModifier::ChooseCardName",
        ]
    );

    let talion = lower_w16c(
        "Name:TalionProbe\nManaCost:2 U B\nTypes:Legendary Creature\nPT:3/4\n\
         K:ETBReplacement:Other:ChooseNumber\n\
         SVar:ChooseNumber:DB$ ChooseNumber | Min$ 1 | Max$ 10 | Defined$ You\n\
         T:Mode$ SpellCast | ValidCard$ Card.cmcEQChosen,Card.powerEQChosen,Card.toughnessEQChosen | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert_eq!(
        talion.enter_modifiers,
        ["EnterModifier::ChooseNumber { min: 1, max: 10 }"]
    );
    assert!(
        talion.statics.contains("Filter::CmcEqualsChosenNumber")
            && talion.statics.contains("Filter::PowerEqualsChosenNumber")
            && talion
                .statics
                .contains("Filter::ToughnessEqualsChosenNumber"),
        "{}",
        talion.statics
    );
}

#[test]
fn choose_player_random_offerings_and_choose_number_variable_payments() {
    // 1. Ruhan of the Fomori: ChoosePlayerAtRandom + UntilEndOfCombat MustAttackPlayer.
    let ruhan = lower_w16c(
        "Name:RuhanProbe\nManaCost:1 U R W\nTypes:Legendary Creature\nPT:7/7\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ TrigChoose | TriggerZones$ Battlefield\n\
         SVar:TrigChoose:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | Random$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Self | ExileOnMoved$ Battlefield | StaticAbilities$ AttackChosen | Duration$ UntilEndOfCombat\n\
         SVar:AttackChosen:Mode$ MustAttack | ValidCreature$ Card.IsRemembered | MustAttack$ ChosenPlayer",
    );
    assert!(
        ruhan.abilities[0].contains("Effect::ChoosePlayerAtRandom { among: PlayerRel::EachOpponent }")
            && ruhan.abilities[0].contains("Modifier::MustAttackPlayer(PlayerRel::Chosen), duration: Duration::UntilEndOfCombat"),
        "{:?}",
        ruhan.abilities
    );

    // 2. Benevolent Offering: two sequential ChoosePlayerFrom choices + .Chosen,You + ChosenCtrl.
    let offering = lower_w16c(
        "Name:OfferingProbe\nManaCost:3 W\nTypes:Instant\n\
         A:SP$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenAmount$ 3 | TokenScript$ w_1_1_spirit_flying | TokenOwner$ .Chosen,You | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | SubAbility$ DBGainLifeYou\n\
         SVar:DBGainLifeYou:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBGainLifeOpp\n\
         SVar:DBGainLifeOpp:DB$ GainLife | Defined$ ChosenPlayer | LifeAmount$ Y\n\
         SVar:X:Count$Valid Creature.YouCtrl/Twice\n\
         SVar:Y:Count$Valid Creature.ChosenCtrl/Twice",
    );
    assert_eq!(
        offering.abilities[0]
            .matches("Effect::ChoosePlayerFrom")
            .count(),
        2,
        "{:?}",
        offering.abilities
    );

    // 3. Die Young & Aether Refinery: variable energy payment chains.
    let die_young = lower_w16c(
        "Name:DieYoungProbe\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 2 | SubAbility$ DBChooseNumber\n\
         SVar:DBChooseNumber:DB$ ChooseNumber | Max$ Max | ListTitle$ amount of energy to pay | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True | UnlessCost$ PayEnergy<X> | UnlessPayer$ You | UnlessSwitched$ True\n\
         SVar:Max:Count$YourCountersEnergy\n\
         SVar:X:Count$ChosenNumber",
    );
    assert!(
        die_young.abilities[0].contains("pay_cost: Some(VariablePayKind::Energy)")
            && die_young.abilities[0].contains("Amount::Negated(&Amount::ResolutionChosenNumber)"),
        "{:?}",
        die_young.abilities
    );

    let refinery = lower_w16c(
        "Name:RefineryProbe\nManaCost:4 R R\nTypes:Artifact\n\
         A:AB$ PutCounter | Cost$ T | Defined$ You | CounterType$ ENERGY | SubAbility$ DBChooseNumber\n\
         SVar:DBChooseNumber:DB$ ChooseNumber | Max$ Count$YourCountersEnergy | ListTitle$ amount of energy to pay | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | ConditionCheckSVar$ X | UnlessCost$ Mandatory PayEnergy<X> | UnlessPayer$ You | UnlessSwitched$ True | TokenScript$ b_x_x_aetherborn | TokenPower$ X | TokenToughness$ X\n\
         SVar:X:Count$ChosenNumber",
    );
    assert!(
        refinery.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::ResolutionChosenNumber, 1)"),
        "{:?}",
        refinery.abilities
    );
}

#[test]
fn wave16c_honest_refusal_guards() {
    let doll = lower_w16c(
        "Name:DollProbe\nManaCost:5\nTypes:Artifact Creature\nPT:0/1\n\
         K:ETBReplacement:Other:ChooseP\n\
         SVar:ChooseP:DB$ ChoosePlayer | Defined$ You | Choices$ Player\n\
         A:AB$ DealDamage | Cost$ T | NumDmg$ 1\n\
         T:Mode$ DamageDoneOnce | Execute$ TrigDamage | ValidTarget$ Card.Self | TriggerZones$ Battlefield\n\
         SVar:TrigDamage:DB$ DealDamage | NumDmg$ X | Defined$ ChosenPlayer\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    assert!(
        doll.abilities.iter().any(|a| a.contains(
            "Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::ThisObject }"
        )) && doll
            .abilities
            .iter()
            .any(|a| a.contains("TargetSpec::Player(PlayerRel::ChosenBySource)")),
        "{:?}",
        doll.abilities
    );

    for bad in [
        // As-enters random ChooseNumber (Haktos) must stay refused until random ETB number choice exists.
        "Name:BadRandomEtbNum\nManaCost:2 W W\nTypes:Creature\nPT:6/1\n\
         K:ETBReplacement:Other:ChooseNum\n\
         SVar:ChooseNum:DB$ ChooseNumber | Min$ 2 | Max$ 4 | Defined$ You | Random$ True",
        // Secretly$ True ChoosePlayer (Emissary of Grudges) must stay refused.
        "Name:BadSecretEtbPlayer\nManaCost:5 R\nTypes:Creature\nPT:6/5\n\
         K:ETBReplacement:Other:ChooseP\n\
         SVar:ChooseP:DB$ ChoosePlayer | Defined$ You | Choices$ Opponent | Secretly$ True",
        // Mismatched UnlessCost$ on ChooseNumber energy payment must refuse.
        "Name:BadWrongUnlessPayer\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ ChooseNumber | Max$ Max | ListTitle$ amount of energy to pay | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ X | UnlessCost$ PayEnergy<X> | UnlessPayer$ Opponent | UnlessSwitched$ True\n\
         SVar:Max:Count$YourCountersEnergy\n\
         SVar:X:Count$ChosenNumber",
        // DayTime$ Day (instead of Neither) on Moved replacement must refuse.
        "Name:BadDayTimeMoved\nManaCost:2 R\nTypes:Creature\nPT:2/3\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | DayTime$ Day | ReplaceWith$ DoNight | ReplacementResult$ Updated\n\
         SVar:DoNight:DB$ DayTime | Value$ Night",
        // Pre-2018-errata ChosenPlayer DealDamage whose Oracle also allows a planeswalker (Vial Smasher) must refuse.
        "Name:BadVialSmasher\nManaCost:1 B R\nTypes:Legendary Creature\nPT:2/3\n\
         T:Mode$ SpellCast | ValidActivatingPlayer$ You | ActivatorThisTurnCast$ EQ1 | ValidCard$ Card | Execute$ TrigChoose | TriggerZones$ Battlefield\n\
         SVar:TrigChoose:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | Random$ True | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | Defined$ ChosenPlayer | NumDmg$ 2 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenPlayer$ True\n\
         Oracle:Whenever you cast your first spell each turn, choose an opponent at random. BadVialSmasher deals 2 damage to that player or a planeswalker that player controls.",
        // Untargeted spell DealDamage omitting both ValidTgts$ and Defined$ must refuse.
        "Name:BadUntargetedSpellDmg\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ 3",
    ] {
        assert!(refused_w16c(bad), "must refuse: {bad}");
    }
}
