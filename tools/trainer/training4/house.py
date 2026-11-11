"""Distillation from the house expert (`--distill-teacher house`, `house_distillation_v1`).

The data side writes H, the gen-7 house expert's answer computed with foresight
on a fork of the game for the student's seat (fair levels only), as the label of
every decision the recorded seat answers: `single_idx` for single choices and
priority, the candidate and pair target masks (with their free masks) for
subsets and pairs, `scalar_target` for numbers; loss weight 1 there and 0 on
every other decision. Header bit 7 (`n_ne_r`) marks where N, r1's own answer,
differs from H. At priority the coarse-class sidecar holds H's and N's classes
(`training4.coarse`); without it the candidates' action columns give pass
(PassPriority, CancelCast) against act.

`house_loss` teaches every kind, each term the mean over its rows:
- N != H at priority, where H's group (act or pass) differs from N's: H acts,
  -log(p_best_act / (p_best_act + p_pass)) (`ce_h_group`; round 0's overfit
  showed that the act group's summed probability rises while it stays spread
  over equivalent taps, so a pass keeps the top-1 the net plays); H passes,
  -log P(pass group). Inside H's group, wherever it offers more than one answer,
  H's exact candidate, -log(p_H / P(H's group)) (`ce_h_within`): which spell,
  which source; it moves no mass between the groups.
- N != H at every other single choice (mulligan, targets, cards, modes, mana
  abilities, ...): -log p_H (`ce_h_single`).
- N != H at subsets (attackers, multiple targets, cards) and pairs (blocks):
  binary cross-entropy to H's mask on every free entry (`bce_h_subset`,
  `bce_h_pair`).
- N != H at scalars (numbers): squared error to H's normalized value
  (`sq_h_scalar`).
- N = H: KL to r1 at every head (`kl_distilled_n_eq_r`, `kl_subset`, `kl_pair`,
  and `sq_scalar` to r1's number), times `kl_weight`.
- `sq_value_r1`: the value readout's squared distance to r1's at every frame,
  times `value_weight` (`--distill-value-weight`); it keeps a trainable trunk's
  value readout where the frozen value head expects it.
Per pending kind the H terms' sum and weight go into the statistics' kind block
(`distill.kind_means`), so `[distill]` lines show every kind apart.

`HouseCounter` is the held-out side: per kind, the share of decisions whose
decoded answer (as `distill.agreement` decodes it) is H's, at N != H and at
N = H, for the student and r1; with the chain counter's rates it gives the gate
(`house_gate`).
"""
from __future__ import annotations

import torch
import torch.nn.functional as F

from .distill import (KIND_SLOTS, PRIORITY, TERMS, _bernoulli_kl, coarse_classes, kind_name)
from .coarse import CLASSES

PASS = CLASSES.index("pass")
H_TERMS = ("ce_h_group", "ce_h_within", "ce_h_single", "bce_h_subset", "bce_h_pair", "sq_h_scalar")
R1_TERMS = ("kl_distilled_n_eq_r", "kl_subset", "kl_pair", "sq_scalar")
GATE_TOP1 = 0.70        # held-out top-1 agreement with H at N != H over every kind
GATE_CHAIN_RATIO = 0.9  # the student's chain-start act rate against H's
GATE_ACTIVE_RATIO = 1.3  # above this the verdict flags a student more active than the teacher


def house_loss(prepared, reference, batch, plans, scope, extra=None):
    """(loss, stats [distill.STATS_WIDTH]) of the house teacher; see the module."""
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    weight = torch.where(valid, get("loss_weight"), 0.)
    decision = get("is_decision") & valid & (weight > 0)
    heads, differs = get("head"), get("n_ne_r")
    kinds = get("pending_kind").clamp(0, KIND_SLOTS - 1)
    index = {name: i for i, name in enumerate(TERMS)}
    sums, dens = torch.zeros(len(TERMS), device=device), torch.zeros(len(TERMS), device=device)
    kind_sums, kind_dens = torch.zeros(KIND_SLOTS, device=device), torch.zeros(KIND_SLOTS, device=device)

    def add(name, rows, values, w):
        """Rows [B] (or entries [B, W]) with their values and weights."""
        w = w * rows
        sums[index[name]] += (torch.where(rows, values, 0.) * w).sum()
        dens[index[name]] += w.sum()

    def add_kind(rows, values, w, t):
        w = w * rows
        kind_sums.index_add_(0, kinds[:, t], torch.where(rows, values, 0.) * w)
        kind_dens.index_add_(0, kinds[:, t], w)

    for t, plan in enumerate(plans):
        w = weight[:, t]
        for head in range(4):
            if not plan.heads[head]:
                continue
            selected = decision[:, t] & (heads[:, t] == head)
            taught, kept = selected & differs[:, t], selected & ~differs[:, t]
            if head == 0:
                student, ref = prepared[t][1].float(), reference[t][1].float()
                width = student.shape[1]
                mask = get("cand_mask")[:, t, :width]
                genuine = mask.sum(-1) > 1
                taught, kept = taught & genuine, kept & genuine
                log_s = torch.log_softmax(torch.where(mask, student, -1e9), -1)
                log_r = torch.log_softmax(torch.where(mask, ref, -1e9), -1)
                add("kl_distilled_n_eq_r", kept, torch.where(mask, log_r.exp() * (log_r - log_s), 0.).sum(-1), w)
                # Rows of other heads carry no single choice; read index 0 there.
                h_idx = torch.where(selected, get("single_idx")[:, t], 0).clamp(0, width - 1)
                own = -log_s.gather(1, h_idx[:, None])[:, 0]
                priority = kinds[:, t] == PRIORITY
                acting = mask & (coarse_classes(batch, t, width).to(device, non_blocking=True) != PASS)
                passing = mask & ~acting
                n_idx = torch.where(mask, ref, -1e9).argmax(-1)
                if "coarse_n_idx" in batch:   # N's recorded answer where the sidecar has it
                    recorded = get("coarse_n_idx")[:, t]
                    n_idx = torch.where(recorded >= 0, recorded, n_idx).clamp(0, width - 1)
                h_act = acting.gather(1, h_idx[:, None])[:, 0]
                n_act = acting.gather(1, n_idx[:, None])[:, 0]
                split = taught & priority & (h_act != n_act)
                best = torch.where(acting, log_s, -1e9).amax(-1)
                pass_mass = torch.logsumexp(torch.where(passing, log_s, -1e9), -1)
                group = torch.where(h_act, torch.logaddexp(best, pass_mass) - best, -pass_mass)
                inside = torch.where(h_act[:, None], acting, passing)
                within = torch.logsumexp(torch.where(inside, log_s, -1e9), -1) + own
                choosing = taught & priority & (inside.sum(-1) > 1)
                single = taught & ~priority
                add("ce_h_group", split, group, w)
                add("ce_h_within", choosing, within, w)
                add("ce_h_single", single, own, w)
                at_priority = torch.where(split, group, 0.) + torch.where(choosing, within, 0.)
                add_kind(taught, torch.where(priority, at_priority, own), w, t)
            elif head in (1, 2):
                prefix = "cand" if head == 1 else "pair"
                student, ref = prepared[t][head].float(), reference[t][head].float()
                width = student.shape[1]
                eligible = get(prefix + "_mask")[:, t, :width] & (get(prefix + "_free_mask")[:, t, :width] > 0)
                free = torch.where(eligible, get(prefix + "_free_mask")[:, t, :width], 0.)
                target = (get(prefix + "_target_mask")[:, t, :width] > .5).float()
                logits, theirs = torch.where(eligible, student, 0.), torch.where(eligible, ref, 0.)
                bce = F.binary_cross_entropy_with_logits(logits, target, reduction="none")
                h_term, r1_term = ("bce_h_subset", "kl_subset") if head == 1 else ("bce_h_pair", "kl_pair")
                entries = free * w[:, None]
                add(h_term, taught[:, None] & eligible, bce, entries)
                add(r1_term, kept[:, None] & eligible, _bernoulli_kl(logits, theirs), entries)
                per_row = (bce * free).sum(-1) / free.sum(-1).clamp(min=1e-6)
                add_kind(taught & eligible.any(-1), per_row, w, t)
            else:
                number = prepared[t][3].float()
                to_h = (number - get("scalar_target")[:, t]).square()
                add("sq_h_scalar", taught, to_h, w)
                add("sq_scalar", kept, (number - reference[t][3].float()).square(), w)
                add_kind(taught, to_h, w, t)
        add("sq_value_r1", valid[:, t], (prepared[t][4].float() - reference[t][4].float()).square(),
            torch.ones_like(w))
    means = sums / dens.clamp(min=1e-6)
    total = (sum(means[index[name]] for name in H_TERMS)
             + scope.kl_weight * sum(means[index[name]] for name in R1_TERMS))
    if scope.value_weight:
        total = total + scope.value_weight * means[index["sq_value_r1"]]
    for function in (extra or {}).values():
        total = total + function(prepared, batch, plans)
    return total, torch.cat((sums.detach(), dens.detach(), kind_sums.detach(), kind_dens.detach()))


# Per kind: rows at N != H, of them the student's and r1's decoded answer is H's;
# then the same at N = H.
AGREEMENT_COLUMNS = ("n_ne_h", "student_n_ne_h", "r1_n_ne_h", "n_eq_h", "student_n_eq_h", "r1_n_eq_h")


def decoded_agreement(outputs, get, t, head, scalar_tolerance):
    """[B] rows the head decides at step t and [B] whether `outputs`' decoded answer is H's:
    single choices by argmax, subsets and pairs by the threshold-0 decode on every free
    entry, numbers within `scalar_tolerance` (as `distill.agreement`)."""
    if head == 0:
        logits = outputs[t][1].float()
        mask = get("cand_mask")[:, t, :logits.shape[1]]
        return mask.sum(-1) > 1, torch.where(mask, logits, -1e9).argmax(-1) == get("single_idx")[:, t]
    if head in (1, 2):
        prefix = "cand" if head == 1 else "pair"
        logits = outputs[t][head].float()
        width = logits.shape[1]
        eligible = get(prefix + "_mask")[:, t, :width] & (get(prefix + "_free_mask")[:, t, :width] > 0)
        target = get(prefix + "_target_mask")[:, t, :width] > .5
        return eligible.any(-1), ~(eligible & ((logits > 0) != target)).any(-1)
    number = outputs[t][3].float()
    return torch.ones_like(number, dtype=torch.bool), (number - get("scalar_target")[:, t]).abs() <= scalar_tolerance


class HouseCounter:
    """Held-out agreement with H per pending kind, for the student and r1."""

    def __init__(self, scalar_tolerance):
        self.scalar_tolerance = scalar_tolerance
        self.table = None

    def observe(self, prepared, reference, batch, plans):
        device = prepared[0][4].device
        get = lambda key: batch[key].to(device, non_blocking=True)
        valid = get("step_valid")
        decision = get("is_decision") & valid & (get("loss_weight") > 0)
        heads, differs = get("head"), get("n_ne_r")
        kinds = get("pending_kind").clamp(0, KIND_SLOTS - 1)
        if self.table is None:
            self.table = torch.zeros(KIND_SLOTS, len(AGREEMENT_COLUMNS), dtype=torch.float64, device=device)
        for t, plan in enumerate(plans):
            for head in range(4):
                if not plan.heads[head]:
                    continue
                decides, mine = decoded_agreement(prepared, get, t, head, self.scalar_tolerance)
                _, theirs = decoded_agreement(reference, get, t, head, self.scalar_tolerance)
                rows = decision[:, t] & (heads[:, t] == head) & decides
                apart, same = rows & differs[:, t], rows & ~differs[:, t]
                counts = torch.stack((apart, apart & mine, apart & theirs, same, same & mine, same & theirs), 1)
                self.table.index_add_(0, kinds[:, t], counts.double())

    def metrics(self, chain=None):
        """`val_house_*`: agreement over every kind and per kind; with the chain counter's
        metrics (`distill.ChainCounter`, R := H), the chain-start act rates, the pass
        shares at priority, chain completion and the gate."""
        table = (self.table.cpu() if self.table is not None
                 else torch.zeros(KIND_SLOTS, len(AGREEMENT_COLUMNS), dtype=torch.float64))
        share = lambda hits, rows: float(hits) / float(rows) if rows else None
        by_kind = {}
        for code in range(KIND_SLOTS):
            n_ne, s_ne, r_ne, n_eq, s_eq, r_eq = table[code].tolist()
            if n_ne or n_eq:
                by_kind[kind_name(code)] = dict(n_ne_h=int(n_ne), student=share(s_ne, n_ne), r1=share(r_ne, n_ne),
                                                n_eq_h=int(n_eq), student_n_eq_h=share(s_eq, n_eq),
                                                r1_n_eq_h=share(r_eq, n_eq))
        n_ne, s_ne, r_ne, n_eq, s_eq, r_eq = table.sum(0).tolist()
        out = dict(val_house_n_ne_h=int(n_ne), val_house_top1_n_ne_h=share(s_ne, n_ne),
                   val_house_top1_n_ne_h_r1=share(r_ne, n_ne), val_house_n_eq_h=int(n_eq),
                   val_house_top1_n_eq_h=share(s_eq, n_eq), val_house_top1_n_eq_h_r1=share(r_eq, n_eq),
                   val_house_top1_n_ne_h_by_kind=by_kind,
                   # The drift watch: where r1 already answers as H, how often the student still does.
                   val_house_top1_n_eq_h_by_kind={name: row["student_n_eq_h"] for name, row in by_kind.items()
                                                  if row["n_eq_h"]})
        chain = chain or {}
        out.update(
            val_house_chain_start_act=chain.get("val_distill_chain_opportunity_student_act"),
            val_house_chain_start_act_h=chain.get("val_distill_chain_opportunity_r_act"),
            val_house_chain_start_act_r1=chain.get("val_distill_chain_opportunity_r1_act"),
            val_house_chain_start_chances=chain.get("val_distill_chain_opportunity_decisions"),
            val_house_chain_start_recall=chain.get("val_distill_chain_start_student_act"),
            val_house_chain_start_recall_r1=chain.get("val_distill_chain_start_r1_act"),
            val_house_chain_completion=chain.get("val_distill_chain_completion"),
            val_house_chain_completion_r1=chain.get("val_distill_chain_completion_r1"),
            val_house_priority_decisions=chain.get("val_distill_priority_decisions"),
            val_house_priority_pass_share=chain.get("val_distill_priority_pass_share_student"),
            val_house_priority_pass_share_h=chain.get("val_distill_priority_pass_share_r"),
            val_house_priority_pass_share_r1=chain.get("val_distill_priority_pass_share_r1"))
        out["val_house_gate"] = house_gate(out)[1]
        return out


def house_gate(metrics, min_top1=GATE_TOP1, min_ratio=GATE_CHAIN_RATIO, active_ratio=GATE_ACTIVE_RATIO):
    """(passed, verdict): held-out top-1 agreement with H at N != H over every kind of at
    least `min_top1`, and the student's chain-start act rate (where H's last priority
    answer was a pass) at least `min_ratio` times H's own on the same rows. One-sided:
    passivity was round 0's failure. Above `active_ratio` times H's the verdict says
    `more_active_than_teacher` and passes; the play check judges whether that is reckless."""
    top1 = metrics.get("val_house_top1_n_ne_h")
    mine, theirs = metrics.get("val_house_chain_start_act"), metrics.get("val_house_chain_start_act_h")
    ratio = mine / theirs if mine is not None and theirs else None
    passed = top1 is not None and top1 >= min_top1 and ratio is not None and ratio >= min_ratio
    return passed, dict(passed=passed, min_top1=min_top1, min_ratio=min_ratio, top1_n_ne_h=top1,
                        top1_n_ne_h_r1=metrics.get("val_house_top1_n_ne_h_r1"),
                        chain_start_act_ratio=ratio, chain_start_act=mine, chain_start_act_h=theirs,
                        more_active_than_teacher=ratio is not None and ratio > active_ratio,
                        chain_start_act_r1=metrics.get("val_house_chain_start_act_r1"),
                        chain_completion=metrics.get("val_house_chain_completion"),
                        chain_completion_r1=metrics.get("val_house_chain_completion_r1"),
                        priority_pass_share=metrics.get("val_house_priority_pass_share"),
                        priority_pass_share_h=metrics.get("val_house_priority_pass_share_h"),
                        priority_pass_share_r1=metrics.get("val_house_priority_pass_share_r1"))
