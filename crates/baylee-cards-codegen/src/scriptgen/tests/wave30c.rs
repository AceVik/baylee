use super::*;

fn cats_w30c() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec!["Goblin".into(), "Wizard".into(), "Fish".into()],
        artifact: vec!["Clue".into(), "Treasure".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn tokens_w30c() -> TokenLookup {
    TokenLookup::held(&[
        (
            "u_1_1_fish",
            "Name:Fish Token\nTypes:Creature Fish\nColors:blue\nPT:1/1",
        ),
        (
            "c_a_treasure_sac",
            "Name:Treasure Token\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any",
        ),
    ])
}

fn read_w30c(text: &str) -> crate::body::CardBody {
    let parsed = parse(text);
    let cats = cats_w30c();
    let tokens = tokens_w30c();
    transcode(&parsed, &cats, Some(&tokens)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, Some(&tokens))
        )
    })
}

fn refused_w30c(text: &str) -> bool {
    let parsed: CardScript = parse(text);
    let cats = cats_w30c();
    let tokens = tokens_w30c();
    transcode(&parsed, &cats, Some(&tokens)).is_none()
}

#[test]
fn prototype_portal_and_soul_foundry_read_cleanly() {
    let portal = read(
        "Name:Prototype Portal\nManaCost:4\nTypes:Artifact\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | \
         OptionalDecider$ You | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Imprint$ True | Origin$ Hand | Destination$ Exile | \
         ChangeType$ Artifact | ChangeNum$ 1\n\
         A:AB$ CopyPermanent | Cost$ X T | Defined$ Imprinted.ExiledWithSource\n\
         SVar:X:Imprinted$CardManaCost\n\
         T:Mode$ ChangesZone | Origin$ Exile | Destination$ Any | Static$ True | \
         ValidCard$ Card.IsImprinted+ExiledWithSource | Execute$ DBForget\n\
         SVar:DBForget:DB$ Bookkeeping | ForgetImprinted$ TriggeredCard\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | ValidCard$ Card.Self | Destination$ Any | \
         Execute$ DBCleanup | Static$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearImprinted$ True\n\
         Oracle:Imprint — When Prototype Portal enters, you may exile an artifact card from your hand.\\n\
         {X}, {T}: Create a token that's a copy of the exiled card. X is the mana value of that card.",
    );
    assert!(
        portal.abilities.iter().any(|a| {
            a.contains("ChooseZone::LinkedExile")
                && a.contains("Effect::CreateTokenCopy")
                && a.contains("x_value = ActivationX::Defined(Amount::TotalManaValueInZone { zone: ZoneSel::LinkedExile, filter: &Filter::Any })")
        }),
        "{:?}",
        portal.abilities
    );
}

#[test]
fn trepanation_blade_reads_cleanly() {
    let blade = read(
        "Name:Trepanation Blade\nManaCost:3\nTypes:Artifact Equipment\n\
         K:Equip:2\n\
         T:Mode$ Attacks | ValidCard$ Card.EquippedBy | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | Defined$ TriggeredDefendingPlayer | Valid$ Land | \
         RevealedDestination$ Graveyard | RememberRevealed$ True | SubAbility$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttackerLKICopy | NumAtt$ +X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.\\nEquip {2}",
    );
    assert!(
        blade.abilities.iter().any(|a| {
            a.contains("Effect::CaptureAffected")
                && a.contains("RepeatDomain::Times(Amount::Fixed(1))")
                && a.contains("Effect::RevealUntilThen { library: PlayerRel::DefendingPlayer")
                && a.contains("Effect::PumpEventObject { power: Amount::RememberedCount")
        }),
        "{:?}",
        blade.abilities
    );
}

#[test]
fn fishing_gear_reads_cleanly() {
    let gear = read_w30c(
        "Name:Fishing Gear\nManaCost:3\nTypes:Artifact Equipment\n\
         K:Equip:2\n\
         T:Mode$ DamageDone | ValidSource$ Creature.EquippedBy | ValidTarget$ Player | \
         CombatDamage$ True | Execute$ TrigExile | TriggerZones$ Battlefield\n\
         SVar:TrigExile:DB$ Dig | Defined$ TriggeredTarget | DigNum$ 1 | ChangeNum$ All | \
         DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Permanent | \
         GainControl$ True | Defined$ Remembered | Origin$ Exile | Optional$ True | Imprint$ True | \
         Destination$ Battlefield | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ u_1_1_fish | TokenOwner$ You | \
         ConditionDefined$ Imprinted | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearImprinted$ True | ClearRemembered$ True\n\
         Oracle:Whenever equipped creature deals combat damage to a player, exile the top card of that player's library. If it's a permanent card, you may put it onto the battlefield under your control. If you don't, create a 1/1 blue Fish creature token.\\nEquip {2}",
    );
    assert!(
        gear.abilities.iter().any(|a| {
            a.contains(
                "Effect::ExileTop { who: PlayerRel::DamagedPlayer, count: Amount::Fixed(1) }",
            ) && a.contains("ZoneSel::ExileAll")
                && a.contains("ChooseMove::Battlefield { tapped: false }")
                && a.contains("for_unchosen:")
        }),
        "{:?}",
        gear.abilities
    );
}

#[test]
fn rashmi_and_ragavan_reads_cleanly() {
    let rashmi = read_w30c(
        "Name:Rashmi and Ragavan\nManaCost:1 G U R\nTypes:Legendary Creature Elf Monkey\nPT:2/4\n\
         T:Mode$ SpellCast | ValidActivatingPlayer$ You | ActivatorThisTurnCast$ EQ1 | \
         PlayerTurn$ True | ValidCard$ Card | Execute$ TrigExile | TriggerZones$ Battlefield\n\
         SVar:TrigExile:DB$ Dig | DigNum$ 1 | ChangeNum$ All | Defined$ Targeted | \
         ValidTgts$ Opponent | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ c_a_treasure_sac | TokenOwner$ You | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered | ValidSA$ Spell.cmcLTX | \
         ValidZone$ Exile | ImprintPlayed$ True | Controller$ You | WithoutManaCost$ True | \
         Optional$ True | Amount$ All | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ STPlay | ForgetOnMoved$ Exile | \
         RememberObjects$ RememberedCard | ConditionDefined$ Imprinted | ConditionPresent$ Card | \
         ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered+nonLand | MayPlay$ True | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl\n\
         Oracle:Whenever you cast your first spell during each of your turns, exile the top card of target opponent's library and create a Treasure token. Then you may cast the exiled card without paying its mana cost if it's a spell with mana value less than the number of artifacts you control. If you don't cast it this way, you may cast it this turn.",
    );
    assert!(
        rashmi.abilities.iter().any(|a| {
            a.contains("Effect::ExileTop { who: PlayerRel::Chosen, count: Amount::Fixed(1) }")
                && a.contains("Effect::Play {")
                && a.contains("ExilePermission::cast(PlayWindow::ThisTurn)")
        }),
        "{:?}",
        rashmi.abilities
    );
}

#[test]
fn audacious_swap_reads_cleanly() {
    let swap = read(
        "Name:Audacious Swap\nManaCost:3 R\nTypes:Instant\nK:Casualty:2\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent.nonEnchantment | Origin$ Battlefield | \
         Destination$ Library | Shuffle$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ Dig | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | \
         Defined$ TargetedOwner | RememberChanged$ True | SubAbility$ DBPutLand\n\
         SVar:DBPutLand:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Land | \
         Defined$ Remembered | DefinedDesc$ it | Origin$ Exile | Destination$ Battlefield | \
         ForgetChanged$ True | SubAbility$ DBCast\n\
         SVar:DBCast:DB$ Play | ConditionDefined$ Remembered | ConditionPresent$ Card | \
         Defined$ Remembered | DefinedDesc$ it | ValidSA$ Spell | Controller$ RememberedOwner | \
         WithoutManaCost$ True | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Casualty 2\\nThe owner of target nonenchantment permanent shuffles it into their library, then exiles the top card of their library. If it's a land card, they put it onto the battlefield. Otherwise, they may cast it without paying its mana cost.",
    );
    assert!(
        swap.abilities[0].contains("Effect::ShuffleTargetsIntoOwnersLibraries")
            && swap.abilities[0].contains("Effect::ExileTop { who: PlayerRel::OwnerOfTarget")
            && swap.abilities[0].contains("owner_control: true")
            && swap.abilities[0].contains("controller: PlayerRel::OwnerOfTarget"),
        "{:?}",
        swap.abilities
    );
}

#[test]
fn skull_storm_reads_cleanly() {
    let storm = read(
        "Name:Skull Storm\nManaCost:7 B B\nTypes:Sorcery\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | Amount$ X\n\
         SVar:X:Count$TotalCommanderCastFromCommandZone\n\
         A:SP$ Sacrifice | Defined$ Opponent | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBRepeatEach\n\
         SVar:DBRepeatEach:DB$ RepeatEach | RepeatPlayers$ OppNonRememberedController | RepeatSubAbility$ DBLoseLife | SubAbility$ DBCleanup\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Remembered | LifeAmount$ PlayerCountRemembered$LifeTotal/HalfUp\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:When you cast this spell, copy it for each time you've cast your commander from the command zone this game.\\nEach opponent sacrifices a creature. Each opponent who can't loses half their life, rounded up.",
    );
    assert!(
        storm.abilities[1].contains("RepeatDomain::EachOpponent")
            && storm.abilities[1].contains(
                "Amount::Half { base: &Amount::LifeTotal(PlayerRel::EventPlayer), round_up: true }"
            ),
        "{:?}",
        storm.abilities
    );
}

#[test]
fn kylox_reads_cleanly() {
    let kylox = read(
        "Name:Kylox, Visionary Inventor\nManaCost:5 U R\nTypes:Legendary Creature Lizard Artificer\nPT:4/4\n\
         K:Menace\nK:Ward:2\nK:Haste\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigSac | TriggerZones$ Battlefield\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Creature.Other | \
         RememberSacrificed$ True | Optional$ True | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | \
         Imprint$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsImprinted+YouOwn | ValidZone$ Exile | \
         ValidSA$ Instant,Sorcery | Controller$ You | WithoutManaCost$ True | Optional$ True | \
         Amount$ All | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:RememberedLKI$CardPower\n\
         SVar:SacX:Count$Valid Creature.Other\n\
         Oracle:Menace, ward {2}, haste\\nWhenever Kylox, Visionary Inventor attacks, sacrifice any number of other creatures, then exile the top X cards of your library, where X is their total power. You may cast any number of instant and/or sorcery spells from among the exiled cards without paying their mana costs.",
    );
    assert!(
        kylox.abilities.iter().any(|a| {
            a.contains("Effect::SacrificeAnyNumber")
                && a.contains("count: Amount::RememberedPower")
                && a.contains("count: PlayCount::All")
        }),
        "{:?}",
        kylox.abilities
    );
}

#[test]
fn braids_arisen_nightmare_reads_cleanly() {
    let braids = read(
        "Name:Braids, Arisen Nightmare\nManaCost:1 B B\nTypes:Legendary Creature Nightmare\nPT:3/3\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Optional$ True | SacValid$ Artifact,Creature,Enchantment,Land,Planeswalker | \
         Amount$ 1 | RememberSacrificed$ True | SubAbility$ DBRepeatEach\n\
         SVar:DBRepeatEach:DB$ RepeatEach | ConditionDefined$ Remembered | ConditionPresent$ Card | \
         RepeatSubAbility$ DBMaySac | RepeatPlayers$ Player.Opponent | SubAbility$ DBCleanup\n\
         SVar:DBMaySac:DB$ Sacrifice | Defined$ Player.IsRemembered | \
         SacValid$ Permanent.sharesCardTypeWith RememberedCard | RememberSacrificed$ True | \
         Optional$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | \
         Defined$ Player.IsRemembered | LifeAmount$ 2 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | Defined$ You | NumCards$ 1\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card.RememberedPlayerCtrl\n\
         Oracle:At the beginning of your end step, you may sacrifice an artifact, creature, enchantment, land, or planeswalker. If you do, each opponent may sacrifice a permanent that shares a card type with it. For each opponent who doesn't, that player loses 2 life and you draw a card.",
    );
    assert!(
        braids.abilities[0].contains("Effect::MayDo")
            && braids.abilities[0].contains("Filter::SharesCardTypeWithRemembered")
            && braids.abilities[0].contains("Effect::Sequence(&["),
        "{:?}",
        braids.abilities
    );
}

#[test]
fn deadly_brew_and_rise_of_the_witch_king_read_cleanly() {
    let brew = read(
        "Name:Deadly Brew\nManaCost:B G\nTypes:Sorcery\n\
         A:SP$ Sacrifice | SacValid$ Creature,Planeswalker | Defined$ Player | \
         RememberSacrificed$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | ConditionDefined$ Remembered | \
         ConditionPresent$ Card.Permanent+YouCtrl | ConditionCompare$ GE1 | Origin$ Graveyard | \
         Destination$ Hand | Hidden$ True | ChangeType$ Permanent.YouCtrl+!IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Each player sacrifices a creature or planeswalker. If you sacrificed a permanent this way, you may return another permanent card from your graveyard to your hand.",
    );
    let rise = read(
        "Name:Rise of the Witch-king\nManaCost:2 B G\nTypes:Sorcery\n\
         A:SP$ Sacrifice | SacValid$ Creature | Defined$ Player | RememberSacrificed$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | ConditionDefined$ Remembered | \
         ConditionPresent$ Card.Permanent+YouCtrl | ConditionCompare$ GE1 | Origin$ Graveyard | \
         Destination$ Battlefield | Hidden$ True | ChangeType$ Permanent.YouCtrl+!IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Each player sacrifices a creature. If you sacrificed a creature this way, you may return another permanent card from your graveyard to the battlefield.",
    );
    assert!(
        brew.abilities[0]
            .contains("Amount::ThisWayBy { what: ThisWay::SacrificedCount, who: PlayerRel::You }")
            && brew.abilities[0].contains("ChooseMove::Hand")
            && rise.abilities[0].contains(
                "Amount::ThisWayBy { what: ThisWay::SacrificedCount, who: PlayerRel::You }"
            )
            && rise.abilities[0].contains("ChooseMove::Battlefield { tapped: false }"),
        "{:?} / {:?}",
        brew.abilities,
        rise.abilities
    );
}

#[test]
fn wave30c_honest_refusal_pins() {
    assert!(
        refused_w30c(
            "Name:BadDreamshaper\nManaCost:5 R\nTypes:Creature\nPT:5/4\n\
             T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDig\n\
             SVar:TrigDig:AB$ DigUntil | Cost$ 2 R Sac<1/Permanent.nonLand/nonland permanent> | \
             Valid$ Permanent.nonLand | FoundDestination$ Battlefield | RevealedDestination$ Library | \
             RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | GainControl$ True\n\
             Oracle:At the beginning of your end step, you may pay {2}{R} and sacrifice a nonland permanent. If you do, reveal cards from the top of your library until you reveal a nonland permanent card. Put that card onto the battlefield and the rest on the bottom of your library in a random order.",
        ),
        "Compound mana+sac optional trigger cost refuses (*Dreamshaper Shaman*)"
    );
    assert!(
        refused_w30c(
            "Name:BadUnlinkedImprintCopy\nManaCost:4\nTypes:Artifact\n\
             A:AB$ CopyPermanent | Cost$ 1 T | Defined$ Imprinted.ExiledWithSource\n\
             A:AB$ ChangeZone | Cost$ 1 ExileFromGrave<1/Creature> | Origin$ Battlefield | Destination$ Exile\n\
             Oracle:{1}, {T}: Copy imprinted card.\\n{1}, Exile a creature card from your graveyard: Exile this artifact.",
        ),
        "CopyPermanent Imprinted.ExiledWithSource refuses when card also has unlinked exile"
    );
    assert!(
        refused_w30c(
            "Name:BadMandatoryGraveyardReturn\nManaCost:B G\nTypes:Sorcery\n\
             A:SP$ Sacrifice | SacValid$ Creature | Defined$ Player | RememberSacrificed$ True | SubAbility$ DBChangeZone\n\
             SVar:DBChangeZone:DB$ ChangeZone | ConditionDefined$ Remembered | \
             ConditionPresent$ Card.Permanent+YouCtrl | ConditionCompare$ GE1 | Origin$ Graveyard | \
             Destination$ Hand | Hidden$ True | ChangeType$ Permanent.YouCtrl+!IsRemembered | SubAbility$ DBCleanup\n\
             SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
             Oracle:Each player sacrifices a creature. If you sacrificed a creature this way, return another permanent card from your graveyard to your hand.",
        ),
        "Deadly Brew shape without Oracle 'you may' refuses instead of guessing min:0"
    );
}
