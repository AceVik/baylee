//! Wave 18b Bookkeeping & special-chain transcoders:
//! - *Day of Black Sun*, *Aetherspouts*, *Corrosive Ooze*, *Creeping Dread*,
//!   *Faerie Artisans*, *Fiendlash*, *Flickerform*, *Gaze of the Gorgon*,
//!   *Sovereigns of Lost Alara*, *Triton Tactics*, *Unforge*
//! - *Eye of Singularity*, *Phyrexian Grimoire*, *Tainted Pact*
//! - *Torment of Venom* (`PutCounter | RememberTargets$ True` -> `GenericChoice`)

use super::chains::LineFlags;
use super::wave27c::CaptureScope;
use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// Entry dispatcher for `DB$/SP$/AB$ Bookkeeping`.
    pub(super) fn wave18b_try_bookkeeping_chain(
        &mut self,
        p: &Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        probe.drop_prose();
        probe.take("StackDescription");

        if self.in_delayed {
            return self.wave18b_delayed_bookkeeping(&mut probe, sub);
        }

        // 1. `Day of Black Sun`: `RememberAll$ Creature.cmcLEX` -> AnimateAll -> DestroyAll -> Cleanup.
        if let Some(valid) = probe.take("RememberAll")
            && probe.exhausted()
            && !probe.has("RememberZone")
            && let Some(effs) = self.wave18b_day_of_black_sun(valid.trim(), sub)
        {
            return Some(effs);
        }

        // 2. `Phyrexian Grimoire`: `RememberAll$ Card.TopGraveyard2+YouCtrl | RememberZone$ Graveyard`.
        if probe.peek("RememberZone") == Some("Graveyard")
            && p.peek("RememberAll") == Some("Card.TopGraveyard2+YouCtrl")
        {
            let mut gp = p.clone();
            gp.drop_prose();
            gp.take("StackDescription");
            gp.take("RememberZone");
            gp.take("RememberAll");
            if gp.exhausted()
                && target.is_none()
                && chain.target.is_none()
                && let Some(effs) = self.wave18b_phyrexian_grimoire(sub, chain)
            {
                return Some(effs);
            }
        }

        // 3. `Aetherspouts`: `ImprintCards$ Valid Creature.attacking`.
        if p.peek("ImprintCards") == Some("Valid Creature.attacking") {
            let mut ap = p.clone();
            ap.drop_prose();
            ap.take("StackDescription");
            ap.take("ImprintCards");
            if ap.exhausted()
                && target.is_none()
                && let Some(effs) = self.wave18b_aetherspouts(sub)
            {
                return Some(effs);
            }
        }

        // 4. `Faerie Artisans`: `ImprintCards$ Remembered` -> CopyPermanent -> Exile non-remembered.
        if p.peek("ImprintCards") == Some("Remembered") {
            let mut fp = p.clone();
            fp.drop_prose();
            fp.take("StackDescription");
            fp.take("ImprintCards");
            if fp.exhausted()
                && target.is_none()
                && let Some(effs) = self.wave18b_faerie_artisans(sub)
            {
                return Some(effs);
            }
        }

        // 5. `Fiendlash`: `RememberObjects$ TriggeredTargetLKICopy` -> DealDamage(Player,Planeswalker).
        if p.peek("RememberObjects") == Some("TriggeredTargetLKICopy") {
            let mut fp = p.clone();
            fp.drop_prose();
            fp.take("StackDescription");
            fp.take("RememberObjects");
            if fp.exhausted()
                && target.is_none()
                && chain.target.is_none()
                && let Some(effs) = self.wave18b_fiendlash(sub, chain)
            {
                return Some(effs);
            }
        }

        // 6. `Sovereigns of Lost Alara`: `RememberObjects$ TriggeredAttacker` -> Search Aura attached.
        if p.peek("RememberObjects") == Some("TriggeredAttacker") {
            let mut sp = p.clone();
            sp.drop_prose();
            sp.take("StackDescription");
            sp.take("RememberObjects");
            if sp.exhausted()
                && target.is_none()
                && let Some(effs) = self.wave18b_sovereigns_of_lost_alara(sub, chain)
            {
                return Some(effs);
            }
        }

        // 7. `Flickerform`: `Defined$ Enchanted | ImprintCards$ Enchanted`.
        if p.peek("Defined") == Some("Enchanted") && p.peek("ImprintCards") == Some("Enchanted") {
            let mut fp = p.clone();
            fp.drop_prose();
            fp.take("StackDescription");
            fp.take("Defined");
            fp.take("ImprintCards");
            if fp.exhausted()
                && target.is_none()
                && let Some(effs) = self.wave18b_flickerform(sub)
            {
                return Some(effs);
            }
        }

        None
    }

    fn wave18b_delayed_bookkeeping(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let rem = p.take("RememberObjects")?;
        if !matches!(
            rem.as_str(),
            "DelayTriggerRemembered" | "DelayTriggerRememberedLKI"
        ) || !p.exhausted()
        {
            return None;
        }
        let s1_name = sub.as_deref()?.trim().to_string();
        let s1_body = self.svars.get(&s1_name)?.clone();
        let (a1, mut p1) = Params::parse(&s1_body)?;
        p1.drop_prose();

        // D1 (`Corrosive Ooze`): `DestroyAll | ValidCards$ Remembered.Equipment+Attached`.
        if a1 == "DestroyAll" && p1.peek("ValidCards") == Some("Remembered.Equipment+Attached") {
            p1.take("ValidCards");
            let tail = p1.take("SubAbility");
            if !p1.exhausted() {
                return None;
            }
            let remaining = self.wave18b_consume_cleanup(tail.as_deref())?;
            let filter = self.body.filter_static(
                "DELAYED",
                "Filter::And(&[Filter::HasSubtype(subtypes::artifact::EQUIPMENT), Filter::AttachedTo(&Filter::Reference)])",
            );
            *sub = remaining;
            return Some(vec![format!(
                "Effect::DestroyAll {{ filter: &{filter}, no_regen: false }}"
            )]);
        }

        // D2 (`Gaze of the Gorgon`): `DestroyAll | ValidCards$ Creature.blockedRemembered,Creature.blockedByRemembered`.
        if a1 == "DestroyAll"
            && p1.peek("ValidCards")
                == Some("Creature.blockedRemembered,Creature.blockedByRemembered")
        {
            p1.take("ValidCards");
            let tail = p1.take("SubAbility");
            if !p1.exhausted() {
                return None;
            }
            let remaining = self.wave18b_consume_cleanup(tail.as_deref())?;
            let filter = self.body.filter_static(
                "DELAYED",
                "Filter::And(&[Filter::CREATURE, Filter::Or(&[Filter::BlockedReferenceThisTurn, Filter::BlockedByReferenceThisTurn])])",
            );
            *sub = remaining;
            return Some(vec![format!(
                "Effect::DestroyAll {{ filter: &{filter}, no_regen: false }}"
            )]);
        }

        // D3 (`Triton Tactics`): `TapAll | ValidCards$ Creature.blockedByRemembered` -> `PumpAll (no untap)`.
        if a1 == "TapAll"
            && p1.take("ValidCards").as_deref() == Some("Creature.blockedByRemembered")
        {
            let s2_name = p1.take("SubAbility")?;
            if !p1.exhausted() {
                return None;
            }
            let s2_body = self.svars.get(s2_name.trim())?.clone();
            let (a2, mut p2) = Params::parse(&s2_body)?;
            p2.drop_prose();
            if a2 != "PumpAll"
                || p2.take("ValidCards").as_deref() != Some("Creature.blockedByRemembered")
                || p2.take("Duration").as_deref() != Some("Permanent")
                || p2.take("KW").as_deref()
                    != Some("HIDDEN This card doesn't untap during your next untap step.")
            {
                return None;
            }
            let tail = p2.take("SubAbility");
            if !p2.exhausted() {
                return None;
            }
            let remaining = self.wave18b_consume_cleanup(tail.as_deref())?;
            let filter = self.body.filter_static(
                "DELAYED",
                "Filter::And(&[Filter::CREATURE, Filter::BlockedByReferenceThisTurn])",
            );
            *sub = remaining;
            return Some(vec![
                format!("Effect::TapAll {{ filter: &{filter} }}"),
                format!(
                    "Effect::SuppressUntap {{ target: TargetSpec::Object(&{filter}), steps: 1 }}"
                ),
            ]);
        }

        None
    }

    fn wave18b_day_of_black_sun(
        &mut self,
        valid: &str,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let s1_body = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1_body)?;
        p1.drop_prose();
        if a1 != "AnimateAll"
            || p1.take("ValidCards").as_deref() != Some("Card.IsRemembered")
            || p1.take("RemoveAllAbilities").as_deref() != Some("True")
        {
            return None;
        }
        let s2_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2_body = self.svars.get(s2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2_body)?;
        p2.drop_prose();
        if a2 != "DestroyAll" || p2.take("ValidCards").as_deref() != Some("Card.IsRemembered") {
            return None;
        }
        let no_regen = match p2.take("NoRegen").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let tail = p2.take("SubAbility");
        if !p2.exhausted() {
            return None;
        }
        let remaining = self.wave18b_consume_cleanup(tail.as_deref())?;
        if remaining.is_some() {
            return None;
        }
        let filter_expr = self.filter_expr(valid)?;
        let filter = self.body.filter_static("CAPTURE", &filter_expr);
        *sub = None;
        Some(vec![format!(
            "Effect::CaptureMatching {{ \
                 filter: &{filter}, \
                 then: &[ \
                     Effect::continuous(&Filter::IsRemembered, Modifier::LoseAllAbilities, Duration::UntilEndOfTurn), \
                     Effect::DestroyAll {{ filter: &Filter::IsRemembered, no_regen: {no_regen} }} \
                 ] \
             }}"
        )])
    }

    fn wave18b_aetherspouts(&mut self, sub: &mut Option<String>) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "RepeatEach" || p1.take("RepeatPlayers").as_deref() != Some("ImprintedOwner") {
            return None;
        }
        let r2_name = p1.take("RepeatSubAbility")?;
        let cleanup_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(r2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        if a2 != "RepeatEach"
            || p2.take("RepeatCards").as_deref() != Some("Card.IsImprinted+RememberedPlayerOwn")
            || p2.take("ChooseOrder").as_deref() != Some("RememberedPlayer")
        {
            return None;
        }
        let ch_name = p2.take("RepeatSubAbility")?;
        if !p2.exhausted() {
            return None;
        }
        let s3 = self.svars.get(ch_name.trim())?.clone();
        let (a3, mut p3) = Params::parse(&s3)?;
        p3.drop_prose();
        if a3 != "GenericChoice"
            || p3.take("Defined").as_deref() != Some("RememberedOwner")
            || p3.take("ShowCurrentCard").as_deref() != Some("Remembered")
        {
            return None;
        }
        let choices = p3.take("Choices")?;
        if !p3.exhausted() {
            return None;
        }
        let choice_names: Vec<&str> = choices.split(',').map(str::trim).collect();
        if choice_names.len() != 2 {
            return None;
        }
        let mut saw_top = false;
        let mut saw_bot = false;
        for name in choice_names {
            let cb = self.svars.get(name)?.clone();
            let (ca, mut cp) = Params::parse(&cb)?;
            cp.drop_prose();
            if ca != "ChangeZone"
                || cp.take("Defined").as_deref() != Some("RememberedCard")
                || cp.take("Origin").as_deref() != Some("Battlefield")
                || cp.take("Destination").as_deref() != Some("Library")
            {
                return None;
            }
            match cp.take("LibraryPosition").as_deref() {
                Some("0") => saw_top = true,
                Some("-1") => saw_bot = true,
                _ => return None,
            }
            if !cp.exhausted() {
                return None;
            }
        }
        if !(saw_top && saw_bot) {
            return None;
        }
        // Validate trailing `Cleanup | ClearImprinted$ True`.
        let cl_body = self.svars.get(cleanup_name.trim())?.clone();
        let (cla, mut clp) = Params::parse(&cl_body)?;
        clp.drop_prose();
        if cla != "Cleanup" || clp.take("ClearImprinted").as_deref() != Some("True") {
            return None;
        }
        let remaining = clp.take("SubAbility");
        if !clp.exhausted() || remaining.is_some() {
            return None;
        }
        *sub = None;
        Some(vec![
            "Effect::CaptureMatching { \
                 filter: &Filter::ATTACKING_CREATURE, \
                 then: &[Effect::ForEachRemembered { \
                     filter: &Filter::Any, \
                     effects: &[Effect::OwnerPutsOnTopOrBottom { target: TargetSpec::EventObject }], \
                 }] \
             }"
            .to_string(),
        ])
    }

    fn wave18b_faerie_artisans(&mut self, sub: &mut Option<String>) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "CopyPermanent"
            || p1.take("RememberTokens").as_deref() != Some("True")
            || p1.peek("Defined") != Some("TriggeredCardLKICopy")
            || p1.peek("Controller") != Some("You")
        {
            return None;
        }
        let s2_name = p1.take("SubAbility")?;
        let s2 = self.svars.get(s2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        if a2 != "ChangeZoneAll"
            || p2.take("Origin").as_deref() != Some("Battlefield")
            || p2.take("Destination").as_deref() != Some("Exile")
            || p2.take("ChangeType").as_deref() != Some("Card.IsImprinted")
            || !p2.exhausted()
        {
            return None;
        }
        let mut copy_effs = self.effect_of_inner(
            "CopyPermanent",
            &mut p1,
            None,
            LineFlags {
                targets_here: false,
                targets_a_player: false,
                single_target: true,
                prior_target_used: false,
            },
        )?;
        if copy_effs.len() != 1 || !p1.exhausted() {
            return None;
        }
        let copy_eff = copy_effs.pop()?;
        let filter = self.body.filter_static(
            "ARTISAN",
            "Filter::And(&[Filter::IsToken, Filter::CreatedBySource, Filter::NotRemembered])",
        );
        *sub = None;
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
                 first: &[{copy_eff}], \
                 then: &[Effect::ExileAll {{ filter: &{filter}, linked: false }}] \
             }}"
        )])
    }

    fn wave18b_fiendlash(
        &mut self,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "DealDamage"
            || p1.take("DamageSource").as_deref() != Some("TriggeredTargetLKICopy")
        {
            return None;
        }
        let valid_tgts = p1.take("ValidTgts")?;
        let num_dmg = p1.take("NumDmg")?;
        if self.svars.get(num_dmg.trim()).map(String::as_str) != Some("RememberedLKI$CardPower") {
            return None;
        }
        let tail = p1.take("SubAbility");
        if !p1.exhausted() {
            return None;
        }
        let remaining = self.wave18b_consume_cleanup(tail.as_deref())?;
        if remaining.is_some() {
            return None;
        }
        let tgt_spec = self.target_spec(&valid_tgts, "DealDamage")?;
        chain.target = Some(tgt_spec.clone());
        *sub = None;
        Some(vec![format!(
            "Effect::EventObjectDealsDamageEqualToPower {{ target: {tgt_spec} }}"
        )])
    }

    fn wave18b_sovereigns_of_lost_alara(
        &mut self,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "ChangeZone"
            || p1.take("Origin").as_deref() != Some("Library")
            || p1.take("Destination").as_deref() != Some("Battlefield")
            || p1.take("ChangeType").as_deref() != Some("Aura.CanEnchantRemembered+YouCtrl")
            || p1.take("AttachedTo").as_deref() != Some("Remembered")
            || !matches!(p1.take("ChangeNum").as_deref(), None | Some("1"))
            || p1.take("Hidden").as_deref() != Some("True")
            || p1.take("ShuffleNonMandatory").as_deref() != Some("True")
        {
            return None;
        }
        let tail = p1.take("SubAbility");
        if !p1.exhausted() {
            return None;
        }
        if self.wave18b_consume_cleanup(tail.as_deref())?.is_some() {
            return None;
        }
        chain.moves_library = true;
        *sub = None;
        Some(vec![
            "Effect::SearchLibraryAttachedToEvent { filter: &Filter::HasSubtype(subtypes::enchantment::AURA), optional: true }"
                .to_string(),
        ])
    }

    fn wave18b_flickerform(&mut self, sub: &mut Option<String>) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        p1.take("StackDescription");
        if a1 != "Bookkeeping"
            || p1.take("RememberAll").as_deref()
                != Some("Card.IsImprinted,Aura.AttachedTo Creature.IsImprinted")
        {
            return None;
        }
        let s2_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(s2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        if a2 != "ChangeZoneAll"
            || p2.take("Origin").as_deref() != Some("Battlefield")
            || p2.take("Destination").as_deref() != Some("Exile")
            || p2.take("ChangeType").as_deref() != Some("Card.IsRemembered")
        {
            return None;
        }
        let s3_name = p2.take("SubAbility")?;
        if !p2.exhausted() {
            return None;
        }
        let s3 = self.svars.get(s3_name.trim())?.clone();
        let (a3, mut p3) = Params::parse(&s3)?;
        p3.drop_prose();
        if a3 != "Effect"
            || p3.take("RememberObjects").as_deref() != Some("Remembered")
            || p3.take("ImprintCards").as_deref() != Some("Imprinted")
            || p3.take("Duration").as_deref() != Some("Permanent")
        {
            return None;
        }
        let trig_name = p3.take("Triggers")?;
        let cleanup_name = p3.take("SubAbility")?;
        if !p3.exhausted() {
            return None;
        }
        let trig_line = self.svars.get(trig_name.trim())?;
        if !trig_line.contains("FlickerformReturn") {
            return None;
        }
        // Consume cleanup if clear remembered+imprinted.
        let cl_body = self.svars.get(cleanup_name.trim())?.clone();
        let (cla, mut clp) = Params::parse(&cl_body)?;
        clp.drop_prose();
        if cla != "Cleanup"
            || clp.take("ClearRemembered").as_deref() != Some("True")
            || clp.take("ClearImprinted").as_deref() != Some("True")
            || !clp.exhausted()
        {
            return None;
        }
        *sub = None;
        Some(vec!["Effect::Flickerform".to_string()])
    }

    fn wave18b_phyrexian_grimoire(
        &mut self,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "ChooseCard"
            || p1.take("ValidTgts").as_deref() != Some("Opponent")
            || p1.take("Choices").as_deref() != Some("Card.IsRemembered")
            || p1.take("ChoiceZone").as_deref() != Some("Graveyard")
            || p1.take("Mandatory").as_deref() != Some("True")
            || p1.take("ForgetChosen").as_deref() != Some("True")
        {
            return None;
        }
        let s2_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(s2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        if a2 != "ChangeZone"
            || p2.take("Defined").as_deref() != Some("ChosenCard")
            || p2.take("Origin").as_deref() != Some("Graveyard")
            || p2.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let s3_name = p2.take("SubAbility")?;
        if !p2.exhausted() {
            return None;
        }
        let s3 = self.svars.get(s3_name.trim())?.clone();
        let (a3, mut p3) = Params::parse(&s3)?;
        p3.drop_prose();
        if a3 != "ChangeZone"
            || p3.take("Defined").as_deref() != Some("Remembered")
            || p3.take("Origin").as_deref() != Some("Graveyard")
            || p3.take("Destination").as_deref() != Some("Hand")
        {
            return None;
        }
        let tail = p3.take("SubAbility");
        if !p3.exhausted() {
            return None;
        }
        if self.wave18b_consume_cleanup(tail.as_deref())?.is_some() {
            return None;
        }
        chain.target = Some("TargetSpec::AnyOpponent".to_string());
        *sub = None;
        Some(vec![
            "Effect::PhyrexianGrimoire { chooser: PlayerRel::Chosen }".to_string(),
        ])
    }

    /// `Creeping Dread`: `Discard | Defined$ Player | NumCards$ 1 | Mode$ TgtChoose | RememberDiscarded$ True`
    /// -> `Bookkeeping (ImprintCards$ ValidAll Card.IsRemembered+YouCtrl)` -> `Forget` -> `RepeatEach` -> `LoseLife(3)`.
    pub(super) fn wave18b_try_creeping_dread(
        &mut self,
        p: &Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        let mut dp = p.clone();
        dp.drop_prose();
        if dp.take("Defined").as_deref() != Some("Player")
            || !matches!(dp.take("NumCards").as_deref(), None | Some("1"))
            || dp.take("Mode").as_deref() != Some("TgtChoose")
            || dp.take("RememberDiscarded").as_deref() != Some("True")
            || !dp.exhausted()
        {
            return None;
        }
        let s1 = self.svars.get(sub?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "Bookkeeping"
            || p1.take("ImprintCards").as_deref() != Some("ValidAll Card.IsRemembered+YouCtrl")
        {
            return None;
        }
        let s2_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(s2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        if a2 != "Bookkeeping" || p2.take("ForgetObjects").as_deref() != Some("Imprinted") {
            return None;
        }
        let s3_name = p2.take("SubAbility")?;
        if !p2.exhausted() {
            return None;
        }
        let s3 = self.svars.get(s3_name.trim())?.clone();
        let (a3, mut p3) = Params::parse(&s3)?;
        p3.drop_prose();
        if a3 != "RepeatEach"
            || p3.take("DefinedCards").as_deref() != Some("Remembered")
            || p3.take("ClearRemembered").as_deref() != Some("True")
        {
            return None;
        }
        let dmg_name = p3.take("RepeatSubAbility")?;
        let cl_name = p3.take("SubAbility")?;
        if !p3.exhausted() {
            return None;
        }
        let dmg_body = self.svars.get(dmg_name.trim())?.clone();
        let (da, mut dmg_p) = Params::parse(&dmg_body)?;
        dmg_p.drop_prose();
        if da != "LoseLife"
            || dmg_p.take("Defined").as_deref() != Some("RememberedController")
            || dmg_p.take("ConditionDefined").as_deref() != Some("Remembered")
            || dmg_p.take("ConditionPresent").as_deref()
                != Some("Card.sharesCardTypeWith Imprinted+OppOwn")
        {
            return None;
        }
        let damage: u16 = dmg_p.take("LifeAmount")?.trim().parse().ok()?;
        if !dmg_p.exhausted() {
            return None;
        }
        let cl_body = self.svars.get(cl_name.trim())?.clone();
        let (cla, mut clp) = Params::parse(&cl_body)?;
        clp.drop_prose();
        if cla != "Cleanup"
            || clp.take("ClearRemembered").as_deref() != Some("True")
            || clp.take("ClearImprinted").as_deref() != Some("True")
            || !clp.exhausted()
        {
            return None;
        }
        Some(vec![format!(
            "Effect::CreepingDread {{ damage: {damage} }}"
        )])
    }

    /// `Tainted Pact` (`Repeat` + `Dig` + optional `Exile -> Hand` + `StoreSVar`).
    pub(super) fn wave18b_try_tainted_pact(
        &mut self,
        p: &Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        let mut rp = p.clone();
        rp.drop_prose();
        rp.take("StackDescription");
        if rp.take("RepeatCheckSVar").as_deref() != Some("TaintedCheck")
            || rp.take("RepeatSVarCompare").as_deref() != Some("EQ0")
            || rp.take("RepeatDefined").as_deref() != Some("Imprinted")
            || rp.take("RepeatPresent").as_deref() != Some("Card.sharesNameWith Remembered")
            || rp.take("RepeatCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let repeat_sub = rp.take("RepeatSubAbility")?;
        if !rp.exhausted() {
            return None;
        }
        let r1 = self.svars.get(repeat_sub.trim())?.clone();
        let (a1, mut p1) = Params::parse(&r1)?;
        if a1 != "Bookkeeping" || p1.take("RememberObjects").as_deref() != Some("Imprinted") {
            return None;
        }
        let f_name = p1.take("SubAbility")?;
        let f_body = self.svars.get(f_name.trim())?.clone();
        let (fa, mut fp) = Params::parse(&f_body)?;
        if fa != "Cleanup" || fp.take("ClearImprinted").as_deref() != Some("True") {
            return None;
        }
        let ex_name = fp.take("SubAbility")?;
        let ex_body = self.svars.get(ex_name.trim())?.clone();
        let (exa, exp) = Params::parse(&ex_body)?;
        if exa != "Dig"
            || exp.peek("DigNum") != Some("1")
            || exp.peek("DestinationZone") != Some("Exile")
            || exp.peek("Imprint") != Some("True")
        {
            return None;
        }
        // Check trailing `DBCleanup` + `DBReset` if present.
        let cl_body = self.svars.get(sub?.trim())?.clone();
        let (cla, mut clp) = Params::parse(&cl_body)?;
        if cla != "Cleanup"
            || clp.take("ClearRemembered").as_deref() != Some("True")
            || clp.take("ClearImprinted").as_deref() != Some("True")
        {
            return None;
        }
        Some(vec!["Effect::TaintedPact".to_string()])
    }

    /// `Unforge`: `TargetOnly (Equipment)` -> `Bookkeeping (Creature.EquippedByTargeted)` -> `Destroy` -> `DealDamage(2)`.
    pub(super) fn wave18b_try_unforge(
        &mut self,
        p: &Params,
        targets_here: bool,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if !targets_here {
            return None;
        }
        let mut tp = p.clone();
        tp.drop_prose();
        tp.take("StackDescription");
        tp.take("IsCurse");
        if !tp.exhausted() {
            return None;
        }
        let s1 = self.svars.get(sub?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        p1.take("StackDescription");
        if a1 != "Bookkeeping"
            || p1.take("RememberAll").as_deref() != Some("Creature.EquippedByTargeted")
        {
            return None;
        }
        let s2_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(s2_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        p2.take("StackDescription");
        if a2 != "Destroy" || p2.take("Defined").as_deref() != Some("Targeted") {
            return None;
        }
        let s3_name = p2.take("SubAbility")?;
        if !p2.exhausted() {
            return None;
        }
        let s3 = self.svars.get(s3_name.trim())?.clone();
        let (a3, mut p3) = Params::parse(&s3)?;
        p3.drop_prose();
        p3.take("StackDescription");
        if a3 != "DealDamage"
            || p3.take("Defined").as_deref() != Some("Remembered")
            || p3.take("ConditionDefined").as_deref() != Some("Remembered")
            || p3.take("ConditionPresent").as_deref() != Some("Creature")
            || p3.take("ConditionCompare").as_deref() != Some("EQ1")
        {
            return None;
        }
        let dmg_raw = p3.take("NumDmg")?;
        let dmg = self.amount_num(&dmg_raw)?;
        let tail = p3.take("SubAbility");
        if !p3.exhausted() {
            return None;
        }
        if self.wave18b_consume_cleanup(tail.as_deref())?.is_some() {
            return None;
        }
        let tgt = chain.target.as_deref()?;
        let filter = self.body.filter_static(
            "UNFORGE",
            "Filter::And(&[Filter::CREATURE, Filter::AttachedHostOfRemembered])",
        );
        Some(vec![format!(
            "Effect::RememberTargetsThen {{ \
                 then: &[Effect::CaptureMatching {{ \
                     filter: &{filter}, \
                     then: &[ \
                         Effect::destroy({tgt}), \
                         Effect::DealDamageEach {{ amount: {dmg}, filter: &Filter::IsRemembered }} \
                     ] \
                 }}] \
             }}"
        )])
    }

    /// `Eye of Singularity` ETB trigger: destroy each nonbasic permanent sharing a name with another.
    pub(super) fn wave18b_try_eye_of_singularity_etb(
        &mut self,
        p: &Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        let mut rp = p.clone();
        rp.drop_prose();
        if rp.take("RepeatCards").as_deref() != Some("Permanent.nonBasic") {
            return None;
        }
        let rem_name = rp.take("RepeatSubAbility")?;
        if !rp.exhausted() {
            return None;
        }
        let rem_body = self.svars.get(rem_name.trim())?.clone();
        let (ra, mut rmp) = Params::parse(&rem_body)?;
        if ra != "Bookkeeping"
            || rmp.take("ImprintCards").as_deref()
                != Some("Valid Permanent.sharesNameWith Remembered+!IsRemembered")
            || !rmp.exhausted()
        {
            return None;
        }
        let d_body = self.svars.get(sub?.trim())?.clone();
        let (da, mut dp) = Params::parse(&d_body)?;
        if da != "DestroyAll"
            || dp.take("ValidCards").as_deref() != Some("Card.IsImprinted")
            || dp.take("NoRegen").as_deref() != Some("True")
        {
            return None;
        }
        let cl_name = dp.take("SubAbility")?;
        if !dp.exhausted() {
            return None;
        }
        let cl_body = self.svars.get(cl_name.trim())?.clone();
        let (cla, mut clp) = Params::parse(&cl_body)?;
        if cla != "Cleanup"
            || clp.take("ClearImprinted").as_deref() != Some("True")
            || !clp.exhausted()
        {
            return None;
        }
        let nonbasic = self.filter_expr("Permanent.nonBasic")?;
        let filter = self.body.filter_static(
            "EYE",
            &format!("Filter::And(&[{nonbasic}, Filter::SharesNameWithAnotherPermanent])"),
        );
        Some(vec![format!(
            "Effect::DestroyAll {{ filter: &{filter}, no_regen: true }}"
        )])
    }

    /// `Eye of Singularity` second trigger: `DestroyAll | ValidCards$ Permanent.!TriggeredCard+sharesNameWith TriggeredCard | NoRegen$ True`.
    pub(super) fn wave18b_try_eye_of_singularity_etb_other(
        &mut self,
        p: &Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        if self.trigger_mode.as_deref() != Some("ChangesZone") || sub.is_some() {
            return None;
        }
        let mut dp = p.clone();
        dp.drop_prose();
        if dp.take("ValidCards").as_deref()
            != Some("Permanent.!TriggeredCard+sharesNameWith TriggeredCard")
            || dp.take("NoRegen").as_deref() != Some("True")
            || !dp.exhausted()
        {
            return None;
        }
        let filter = self.body.filter_static(
            "EYE",
            "Filter::And(&[Filter::NotRemembered, Filter::SharesNameWithRemembered])",
        );
        Some(vec![format!(
            "Effect::CaptureMatching {{ \
                 filter: &Filter::Reference, \
                 then: &[Effect::DestroyAll {{ filter: &{filter}, no_regen: true }}] \
             }}"
        )])
    }

    /// `Torment of Venom`: `PutCounter | RememberTargets$ True | SubAbility$ DBGenericChoice`.
    pub(super) fn wave18b_try_put_counter_remember_targets_chain(
        &mut self,
        p: &Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if !targets_here || p.peek("RememberTargets") != Some("True") {
            return None;
        }
        let mut put_p = p.clone();
        put_p.take("RememberTargets");
        let put_effs = self.effect_of_inner(
            "PutCounter",
            &mut put_p,
            chain.target.as_deref(),
            LineFlags {
                targets_here,
                targets_a_player: false,
                single_target: true,
                prior_target_used: false,
            },
        )?;
        if !put_p.exhausted() {
            return None;
        }
        let choice_name = sub.as_deref()?.trim().to_string();
        let choice_body = self.svars.get(&choice_name)?.clone();
        let (c_api, cp) = Params::parse(&choice_body)?;
        if c_api != "GenericChoice" {
            return None;
        }
        let mut cp_clean = cp;
        let mut choice_sub = cp_clean.take("SubAbility").map(|s| s.trim().to_string());
        let saved_scope = self.capture_scope.replace(CaptureScope {
            single_object: true,
            guaranteed_valid: None,
            from_battlefield: true,
            you_own_only: false,
            multi_player: false,
            captured_zone: None,
            dest_hand_you: false,
        });
        let choice_effs =
            self.wave18b_try_generic_choice_chain(cp_clean, &mut choice_sub, false, chain);
        self.capture_scope = saved_scope;
        let choice_effs = choice_effs?;
        *sub = choice_sub;
        let mut out = put_effs;
        out.push(format!(
            "Effect::RememberTargetsThen {{ then: &[{}] }}",
            choice_effs.join(", ")
        ));
        Some(out)
    }
}
