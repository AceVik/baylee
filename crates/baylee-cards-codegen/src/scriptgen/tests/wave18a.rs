use super::*;

#[test]
fn wave18a_affected_zone_stack_cascade_blitz_and_affinity() {
    let abaddon = read(
        "Name:Abaddon the Despoiler\nManaCost:2 U B R\nTypes:Legendary Creature Astartes Warrior\n\
         PT:5/5\nK:Trample\n\
         S:Mode$ Continuous | Condition$ PlayerTurn | Affected$ Card.YouCtrl+YouOwn+wasCastFromHand+cmcLEX | \
         AffectedZone$ Stack | AddKeyword$ Cascade\n\
         SVar:X:Count$LifeOppsLostThisTurn",
    );
    assert!(
        abaddon.abilities[0]
            .contains("Filter::CmcAtMostAmount(&Amount::OpponentsLifeLostThisTurn)")
            && abaddon.abilities[0].contains("Modifier::GrantCascade(1)")
            && abaddon.abilities[0].contains("Condition::YourTurn"),
        "{:?}",
        abaddon.abilities
    );

    let anhelo = read(
        "Name:Anhelo, the Painter\nManaCost:U B R\nTypes:Legendary Creature Vampire Assassin\nPT:1/3\n\
         S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | AffectedZone$ Stack | \
         CheckSVar$ X | SVarCompare$ EQ0 | AddKeyword$ Casualty:2\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
    );
    assert!(
        anhelo.abilities[0].contains("Filter::FirstMatchingSpellThisTurn(")
            && anhelo.abilities[0].contains("Modifier::GrantCasualty(2)"),
        "{} / {:?}",
        anhelo.statics,
        anhelo.abilities
    );

    let henzie = read(
        "Name:Henzie \"Toolbox\" Torre\nManaCost:B R G\nTypes:Legendary Creature Devil Rogue\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl+wasCast | AffectedZone$ Stack | \
         AddKeyword$ Blitz:CardManaCost:Spell.Creature+cmcGE4\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Spell.Blitz | Activator$ You | Amount$ CommanderCast\n\
         SVar:CommanderCast:Count$TotalCommanderCastFromCommandZone",
    );
    assert!(
        henzie
            .abilities
            .iter()
            .any(|a| a.contains("Filter::CmcAtLeast(4)")
                && a.contains("Modifier::GrantBlitzCardManaCost"))
            && henzie
                .abilities
                .iter()
                .any(|a| a.contains("Filter::CastForBlitz")
                    && a.contains("Amount::CommanderCasts(PlayerRel::You)")),
        "{} / {:?}",
        henzie.statics,
        henzie.abilities
    );

    let pearl_parsed = parse(
        "Name:Pearl-Ear, Imperial Advisor\nManaCost:1 W W\nTypes:Legendary Creature Fox Advisor\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ Enchantment.YouCtrl+wasCast | AffectedZone$ Stack | \
         AddKeyword$ Affinity:Aura",
    );
    let mut p_cats = cats();
    p_cats.enchantment.push("Aura".into());
    p_cats.normalize();
    let pearl_ear = transcode(&pearl_parsed, &p_cats, None).unwrap_or_else(|| {
        panic!(
            "Pearl-Ear should read in full, refused: {:?}",
            refusal_reason(&pearl_parsed, &p_cats, None)
        )
    });
    assert!(
        pearl_ear.abilities[0].contains("Modifier::GrantAffinity(")
            && pearl_ear.statics.contains("AURA"),
        "{} / {:?}",
        pearl_ear.statics,
        pearl_ear.abilities
    );
}

#[test]
fn wave18a_affected_zone_stack_demonstrate_ripple_replicate_delve() {
    let zhulodok = read(
        "Name:Zhulodok, Void Gorger\nManaCost:5 C\nTypes:Legendary Creature Eldrazi\nPT:7/4\n\
         S:Mode$ Continuous | Affected$ Card.Colorless+YouCtrl+YouOwn+wasCastFromHand+cmcGE7 | \
         AffectedZone$ Stack | AddKeyword$ Cascade & Cascade",
    );
    assert!(
        zhulodok.abilities[0].contains("Modifier::GrantCascade(2)"),
        "{:?}",
        zhulodok.abilities
    );

    let twelfth = read(
        "Name:The Twelfth Doctor\nManaCost:3 U R\nTypes:Legendary Creature Time Lord Doctor\nPT:4/4\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl+!wasCastFromYourHand | AffectedZone$ Stack | \
         CheckSVar$ X | SVarCompare$ EQ0 | AddKeyword$ Demonstrate\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl+!wasCastFromYourHand",
    );
    assert!(
        twelfth.abilities[0].contains("KeywordSet::DEMONSTRATE")
            && twelfth.abilities[0].contains("Filter::FirstMatchingSpellThisTurn("),
        "{} / {:?}",
        twelfth.statics,
        twelfth.abilities
    );

    let thrumming = read(
        "Name:Thrumming Stone\nManaCost:5\nTypes:Legendary Artifact\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl+wasCast | AffectedZone$ Stack | AddKeyword$ Ripple:4",
    );
    assert!(
        thrumming.abilities[0].contains("Modifier::GrantRipple(4)"),
        "{:?}",
        thrumming.abilities
    );

    let djinn = read(
        "Name:Djinn Illuminatus\nManaCost:5 UR UR\nTypes:Creature Djinn\nPT:3/5\n\
         S:Mode$ Continuous | AddKeyword$ Replicate:CardManaCost | \
         Affected$ Instant.YouCtrl+wasCast,Sorcery.YouCtrl+wasCast | AffectedZone$ Stack",
    );
    assert!(
        djinn.abilities[0].contains("Modifier::GrantReplicateCardManaCost"),
        "{:?}",
        djinn.abilities
    );

    let teval = read(
        "Name:Teval, Arbiter of Virtue\nManaCost:2 B G U\nTypes:Legendary Creature Spirit Dragon\nPT:6/6\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl+wasCast | AffectedZone$ Stack | AddKeyword$ Delve",
    );
    assert!(
        teval.abilities[0].contains("KeywordSet::DELVE"),
        "{:?}",
        teval.abilities
    );
}

#[test]
fn wave18a_affected_zone_stack_freerunning_prowl_and_honest_refusals() {
    let mut stack_cats = cats();
    stack_cats.creature.push("Assassin".into());
    stack_cats.creature.push("Dinosaur".into());
    stack_cats.normalize();

    let ezio_parsed = parse(
        "Name:Ezio Auditore da Firenze\nManaCost:1 B\nTypes:Legendary Creature Human Assassin\nPT:3/2\n\
         S:Mode$ Continuous | Affected$ Card.Assassin+YouCtrl+wasCast | AffectedZone$ Stack | \
         AddKeyword$ Freerunning:B B",
    );
    let ezio = transcode(&ezio_parsed, &stack_cats, None).unwrap();
    assert!(
        ezio.abilities[0].contains("Modifier::GrantFreerunning(mana!(\"{B}{B}\"))"),
        "{:?}",
        ezio.abilities
    );

    let vel_parsed = parse(
        "Name:Hunting Velociraptor\nManaCost:2 R\nTypes:Creature Dinosaur\nPT:3/2\n\
         S:Mode$ Continuous | Affected$ Dinosaur.YouCtrl+wasCast | AddKeyword$ Prowl:2 R | AffectedZone$ Stack",
    );
    let velociraptor = transcode(&vel_parsed, &stack_cats, None).unwrap();
    assert!(
        velociraptor.abilities[0].contains("Modifier::GrantProwl(mana!(\"{2}{R}\"))"),
        "{:?}",
        velociraptor.abilities
    );

    // Honest refusals: Assist, fixed-mana Replicate, Emerge, Offspring.
    for bad in [
        "Name:Yume, Chronicler of Valor\nManaCost:1 W\nTypes:Legendary Creature Human Druid\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl | Condition$ PlayerTurn | AffectedZone$ Stack | \
         AddKeyword$ Assist | CheckSVar$ X | SVarCompare$ EQ0\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl",
        "Name:Threefold Signal\nManaCost:3\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl+numColorsEQ3 | AffectedZone$ Stack | AddKeyword$ Replicate:3",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave18a_you_pw_triggers_and_cant_attack_target() {
    let revenge = read(
        "Name:Revenge of Ravens\nManaCost:3 B\nTypes:Enchantment\n\
         T:Mode$ Attacks | ValidCard$ Creature | Attacked$ You,Planeswalker.YouCtrl | \
         TriggerZones$ Battlefield | Execute$ TrigLoseLife\n\
         SVar:TrigLoseLife:DB$ LoseLife | Defined$ TriggeredAttackerController | LifeAmount$ 1",
    );
    assert!(
        revenge
            .statics
            .contains("Filter::AttackingYouOrYourPlaneswalker"),
        "{}",
        revenge.statics
    );

    let kazuul_parsed = parse(
        "Name:Kazuul, Tyrant of the Cliffs\nManaCost:3 R R\nTypes:Legendary Creature Ogre Warrior\nPT:5/4\n\
         T:Mode$ Attacks | ValidCard$ Creature.OppCtrl | \
         Attacked$ You,Planeswalker.YouCtrl,Battle.ProtectedBy You | \
         TriggerZones$ Battlefield | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenOwner$ You | TokenScript$ r_3_3_ogre | UnlessCost$ 3 | \
         UnlessPayer$ TriggeredAttackerController",
    );
    let mut k_cats = cats();
    k_cats.creature.push("Ogre".into());
    k_cats.normalize();
    let k_tok = TokenLookup::held(&[(
        "r_3_3_ogre",
        "Name:Ogre\nTypes:Creature Ogre\nColors:red\nPT:3/3",
    )]);
    let kazuul = transcode(&kazuul_parsed, &k_cats, Some(&k_tok)).unwrap_or_else(|| {
        panic!(
            "Kazuul should read in full, refused: {:?}",
            refusal_reason(&kazuul_parsed, &k_cats, Some(&k_tok))
        )
    });
    assert!(
        kazuul.statics.contains("Filter::DefendingPlayerIsYou"),
        "{}",
        kazuul.statics
    );

    let threshold = read(
        "Name:Ever-Watching Threshold\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclared | AttackingPlayer$ Opponent | AttackedTarget$ You,Planeswalker.YouCtrl | \
         TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You",
    );
    assert!(
        threshold.abilities[0].contains(
            "Trigger::AttackersDeclaredAgainst { attacker: PlayerRel::EachOpponent, \
             attacked: AttackedTargetKind::YouOrYourPlaneswalker }"
        ),
        "{:?}",
        threshold.abilities
    );

    let convergence = read(
        "Name:Sandwurm Convergence\nManaCost:6 G G\nTypes:Enchantment\n\
         S:Mode$ CantAttack | ValidCard$ Creature.withFlying | Target$ You,Planeswalker.YouCtrl",
    );
    assert!(
        convergence.abilities[0].contains("Modifier::CantAttackYouOrYourPlaneswalkers"),
        "{:?}",
        convergence.abilities
    );

    let aetherspark = read(
        "Name:The Aetherspark\nManaCost:4\nTypes:Legendary Artifact Planeswalker Equipment\nLoyalty:4\n\
         S:Mode$ CantAttack | ValidCard$ Creature | Target$ Card.Self+AttachedTo Creature",
    );
    assert!(
        aetherspark.abilities[0].contains("Modifier::CantBeAttackedBy(&Filter::CREATURE)")
            && aetherspark.abilities[0].contains("Filter::AttachedTo(&Filter::CREATURE)"),
        "{:?}",
        aetherspark.abilities
    );
}

#[test]
fn wave18a_granted_loyalty_and_teferi_emblem_and_pwwarlord() {
    let rowan = read(
        "Name:Rowan's Talent\nManaCost:2 R R\nTypes:Enchantment Aura\nK:Enchant:Planeswalker\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Planeswalker | AddAbility$ RowanPlus\n\
         SVar:RowanPlus:AB$ Pump | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | ValidTgts$ Creature | \
         TargetMin$ 0 | TargetMax$ 1 | NumAtt$ +2 | KW$ First Strike\n\
         T:Mode$ AbilityCast | ValidCard$ Planeswalker.EnchantedBy+inRealZoneBattlefield | \
         ValidSA$ Activated.Loyalty | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigCopyAbility\n\
         SVar:TrigCopyAbility:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
    );
    assert!(
        rowan.statics.contains("loyalty!(1,")
            && rowan
                .abilities
                .iter()
                .any(|a| a.contains("Modifier::GrantActivatedDefinition"))
            && rowan
                .abilities
                .iter()
                .any(|a| a.contains("Trigger::LoyaltyAbilityActivated")),
        "{} / {:?}",
        rowan.statics,
        rowan.abilities
    );

    let teferi = read(
        "Name:Teferi's Talent\nManaCost:3 U U\nTypes:Enchantment Aura\nK:Enchant:Planeswalker\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Planeswalker | AddAbility$ TeferiUlt\n\
         SVar:TeferiUlt:AB$ Effect | Cost$ SubCounter<12/LOYALTY> | Planeswalker$ True | \
         Name$ Emblem — Teferi, Temporal Archmage | StaticAbilities$ InstantPlaneswalkers | \
         Stackable$ False | Duration$ Permanent\n\
         SVar:InstantPlaneswalkers:Mode$ CastWithFlash | ValidCard$ Planeswalker.YouCtrl | \
         ValidSA$ Activated.Loyalty | Caster$ You\n\
         T:Mode$ Drawn | ValidCard$ Card.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Enchanted | CounterType$ LOYALTY | CounterNum$ 1",
    );
    assert!(
        teferi
            .statics
            .contains("Modifier::MayActivateLoyaltyAsInstant { who: PlayerRel::You }")
            && teferi
                .statics
                .contains("loyalty!(-12, &[Effect::CreateEmblem"),
        "{}",
        teferi.statics
    );

    let warlord = read(
        "Name:Way of the Warlord\nManaCost:2 R\nTypes:Legendary Enchantment\n\
         S:Mode$ Continuous | Affected$ Planeswalker.YouCtrl | AddAbility$ PWWarlord\n\
         SVar:PWWarlord:AB$ DealDamage | Cost$ SubCounter<4/LOYALTY> | Planeswalker$ True | \
         ValidTgts$ Creature,Planeswalker | TargetMin$ 0 | TargetMax$ 1 | NumDmg$ 2 | \
         SubAbility$ DBDealDamage | DamageMap$ True\n\
         SVar:DBDealDamage:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 2 | SubAbility$ DBDamageResolve\n\
         SVar:DBDamageResolve:DB$ DamageResolve",
    );
    assert!(
        warlord.statics.contains("loyalty!(-4,")
            && warlord.statics.contains("second_targets = Some(")
            && warlord.abilities[0].contains("Modifier::GrantActivatedDefinition"),
        "{} / {:?}",
        warlord.statics,
        warlord.abilities
    );
}

#[test]
fn wave18a_tgt_zone_graveyard_cards() {
    let songcrafter = read(
        "Name:Songcrafter Mage\nManaCost:G U R\nTypes:Creature Human Bard\nPT:3/2\nK:Flash\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigFlashback\n\
         SVar:TrigFlashback:DB$ Pump | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | TgtZone$ Graveyard | \
         KW$ Harmonize | PumpZone$ Graveyard | AILogic$ ReplaySpell",
    );
    assert!(
        songcrafter.abilities[0].contains("Modifier::GrantsHarmonizeManaCost"),
        "{:?}",
        songcrafter.abilities
    );

    let jolted = read(
        "Name:Jolted Awake\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 2 | \
         ValidTgts$ Artifact.YouOwn,Creature.YouOwn | TargetMin$ 0 | TargetMax$ 1 | TgtZone$ Graveyard | \
         SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Defined$ Targeted | Origin$ Graveyard | Destination$ Battlefield | \
         UnlessCost$ PayEnergy<X> | UnlessPayer$ You | UnlessSwitched$ True\n\
         SVar:X:Targeted$CardManaCost\nK:Cycling:2",
    );
    assert!(
        jolted.abilities.iter().any(|a| {
            a.contains("Effect::Energy { who: PlayerRel::You, amount: Amount::Fixed(2) }")
                && a.contains("Effect::MayPayEnergyEqualToTargetCmcThen")
        }),
        "{:?}",
        jolted.abilities
    );

    let command = read(
        "Name:Command the Dreadhorde\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | TargetMin$ 0 | TargetMax$ NumCreatPW | ValidTgts$ Creature,Planeswalker | \
         TgtZone$ Graveyard | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | Defined$ You | NumDmg$ X | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ Targeted | GainControl$ True | Origin$ Graveyard | \
         Destination$ Battlefield\n\
         SVar:NumCreatPW:Count$ValidGraveyard Creature,Planeswalker\n\
         SVar:X:Targeted$CardManaCost",
    );
    assert!(
        command.abilities[0].contains("Amount::TotalTargetsManaValue")
            && command.abilities[0].contains("TargetReq::up_to("),
        "{:?}",
        command.abilities
    );

    let rescue = read(
        "Name:Rescue from the Underworld\nManaCost:4 B\nTypes:Instant\n\
         A:SP$ DelayedTrigger | Cost$ 4 B Sac<1/Creature> | ValidTgts$ Creature.YouOwn | TgtZone$ Graveyard | \
         Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ MoveToPlay | \
         RememberObjects$ Targeted & SacrificedCards | SubAbility$ DBExile\n\
         SVar:MoveToPlay:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | \
         Defined$ DelayTriggerRememberedLKI | GainControl$ True\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile",
    );
    assert!(
        rescue.abilities[0].contains("TriggerRemember::TargetAndSacrificed")
            && rescue.abilities[0]
                .contains("Effect::ReturnDelayedObjectsToBattlefield { owner_control: false }")
            && rescue.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        rescue.abilities
    );

    assert!(
        rescue.abilities[0].contains("Remember: TriggerRemember::TargetAndSacrificed")
            || rescue.abilities[0].contains("TriggerRemember::TargetAndSacrificed"),
        "{:?}",
        rescue.abilities
    );
}

#[test]
fn wave18a_tgt_zone_graveyard_mavinda_sins_lich_and_call() {
    let mavinda = read(
        "Name:Mavinda, Students' Advocate\nManaCost:2 W\nTypes:Legendary Creature Bird Advisor\nPT:2/3\n\
         A:AB$ Effect | Cost$ 0 | ValidTgts$ Instant.YouOwn,Sorcery.YouOwn | TgtZone$ Graveyard | \
         ActivationLimit$ 1 | RememberObjects$ Targeted | StaticAbilities$ MayPlay,RaiseCost | \
         ReplacementEffects$ ReplaceGraveyard | SubAbility$ DBCleanup\n\
         SVar:MayPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered+nonLand | \
         AffectedZone$ Graveyard\n\
         SVar:RaiseCost:Mode$ RaiseCost | ValidCard$ Card.IsRemembered+CastSa Spell.MayPlaySource | \
         ValidTarget$ Creature.YouCtrl+inZoneBattlefield | UnlessValidTarget$ True | Activator$ You | \
         Type$ Spell | Amount$ 8\n\
         SVar:ReplaceGraveyard:Event$ Moved | ValidLKI$ Card.IsRemembered+CastSa Spell.MayPlaySource | \
         Destination$ Graveyard | ReplaceWith$ MoveExile\n\
         SVar:MoveExile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | Destination$ Exile | \
         SubAbility$ ExileSelf\n\
         SVar:ExileSelf:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        mavinda.abilities[0].contains(
            "Modifier::MayCastFromGraveyardTargeted { free: false, exile_instead_of_graveyard: true }"
        ) && mavinda.abilities[0].contains(
            "Modifier::CastCostIncreaseUnlessTargets { filter: &Filter::YOUR_CREATURE, generic: 8 }"
        ),
        "{:?}",
        mavinda.abilities
    );

    let sins = read(
        "Name:Sins of the Past\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | TgtZone$ Graveyard | \
         RememberObjects$ Targeted | StaticAbilities$ Play | SubAbility$ DBEffect | ExileOnMoved$ Graveyard\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | MayPlayWithoutManaCost$ True | \
         Affected$ Card.IsRemembered | AffectedZone$ Graveyard\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Targeted | ExileOnMoved$ Stack | \
         ReplacementEffects$ ReplaceGraveyard | SubAbility$ DBChange\n\
         SVar:ReplaceGraveyard:Event$ Moved | ValidCard$ Card.IsRemembered | Origin$ Stack | \
         Destination$ Graveyard | ReplaceWith$ MoveExile\n\
         SVar:MoveExile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Stack | Destination$ Exile\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Stack | Destination$ Exile",
    );
    assert!(
        sins.abilities[0].contains(
            "Modifier::MayCastFromGraveyardTargeted { free: true, exile_instead_of_graveyard: true }"
        ) && sins.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        sins.abilities
    );

    let havengul = read(
        "Name:Havengul Lich\nManaCost:3 U B\nTypes:Creature Zombie Wizard\nPT:4/4\n\
         A:AB$ Effect | Name$ Havengul Lich's Delayed Trigger | Cost$ 1 | ValidTgts$ Creature | \
         TgtZone$ Graveyard | StaticAbilities$ STPlay | Triggers$ DTCast | RememberObjects$ Targeted\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | \
         AffectedZone$ Graveyard\n\
         SVar:DTCast:Mode$ SpellCast | ValidCard$ Card.IsRemembered+nonLand | Execute$ StealAbs\n\
         SVar:StealAbs:DB$ Effect | RememberObjects$ TriggeredCard | StaticAbilities$ STSteal\n\
         SVar:STSteal:Mode$ Continuous | AffectedDefined$ EffectSource | GainsAbilitiesOfDefined$ RememberedLKI",
    );
    assert!(
        havengul.abilities[0].contains(
            "Modifier::MayCastFromGraveyardTargeted { free: false, exile_instead_of_graveyard: false }"
        ) && havengul.abilities[0]
            .contains("Effect::GrantActivatedAbilitiesWhenTargetCast { duration: Duration::UntilEndOfTurn }"),
        "{:?}",
        havengul.abilities
    );

    let spirit_sister = read(
        "Name:Spirit-Sister's Call\nManaCost:3 W B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigChoose\n\
         SVar:TrigChoose:DB$ ChooseCard | ValidTgts$ Permanent.YouOwn | TgtZone$ Graveyard | \
         DefinedCards$ Targeted | SubAbility$ DBSacrifice\n\
         SVar:DBSacrifice:DB$ Sacrifice | Optional$ True | SacValid$ Permanent.sharesCardTypeWith Targeted | \
         Amount$ 1 | RememberSacrificed$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Card | \
         Defined$ Targeted | Origin$ Graveyard | Destination$ Battlefield | SubAbility$ DBAnimate\n\
         SVar:DBAnimate:DB$ Animate | ConditionDefined$ Remembered | ConditionPresent$ Card | \
         Replacements$ ReplaceLeaves | Defined$ Targeted | Duration$ Permanent | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True\n\
         SVar:ReplaceLeaves:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | \
         ValidCard$ Card.Self | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | Defined$ ReplacedCard",
    );
    assert!(
        spirit_sister.abilities[0].contains("Filter::SharesCardTypeWithReference")
            && spirit_sister.abilities[0].contains("Effect::PlayerMaySacrificeNThen")
            && spirit_sister.abilities[0].contains("Modifier::ExileInsteadOfLeavingBattlefield"),
        "{} / {:?}",
        spirit_sister.statics,
        spirit_sister.abilities
    );
}
