"""Playernet v4 Phase A: Self-Supervised Card DSL & Set-Context Pretraining (`pretrain_dsl4.py`).

Implements T3 (Phase A) from `docs/history/trained-ai-v4-handover-2026-10-22.md`:
1. Masked Card DSL Feature Autoencoding (30% feature masking across the 160-dim
   typed `baylee_cards_dsl` representation).
2. Ability-to-Card Contrastive InfoNCE alignment between `card_dsl.bin` (`[N, 160]`)
   and `card_abilities.bin` (`[N, 8, 64]`).
3. Multi-Card Set Transformer Masked Reconstruction (24-card board/hand sets
   passed through `PlayerNet4.trunk_layers`).

Historical encoder-5 experiment, retained for reference. It cannot produce
encoder-6 checkpoints: use `pretrain_structure4.py` for the complete rule DAG.
"""

from __future__ import annotations

import argparse
import time
from pathlib import Path

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

import model4 as M
from torch_io4 import atomic_json

ABILITY_DSL_WIDTH = 64
MAX_ABILITIES = 8


class PhaseAPretrainer(nn.Module):
    def __init__(self, net: M.PlayerNet4):
        super().__init__()
        self.net = net
        d = net.cfg.d_model
        self.recon_head = nn.Sequential(
            nn.Linear(d, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, M.CARD_DSL_WIDTH),
        )
        self.ability_enc = nn.Sequential(
            nn.Linear(ABILITY_DSL_WIDTH, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, d),
        )
        self.card_contrast_proj = nn.Linear(d, d)
        self.abil_contrast_proj = nn.Linear(d, d)


def main() -> None:
    ap = argparse.ArgumentParser(description="Phase A Card DSL Self-Supervised Pretraining")
    ap.add_argument("--data", required=True, help="Dataset directory containing card_dsl.bin and card_abilities.bin")
    ap.add_argument("--out", required=True, help="Output checkpoint directory")
    ap.add_argument("--d-model", type=int, default=512)
    ap.add_argument("--layers", type=int, default=8)
    ap.add_argument("--heads", type=int, default=8)
    ap.add_argument("--steps", type=int, default=400)
    ap.add_argument("--batch-size", type=int, default=128)
    ap.add_argument("--set-size", type=int, default=24)
    ap.add_argument("--lr", type=float, default=5e-4)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    if M.ENCODER_VERSION >= 6:
        ap.error("This legacy eight-ability summary pretrainer is incompatible with encoder 6; use pretrain_structure4.py")

    torch.manual_seed(args.seed)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    use_bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported()

    data_dir = Path(args.data).expanduser()
    dsl_raw = np.fromfile(data_dir / "card_dsl.bin", dtype=np.float32)
    n_cards = len(dsl_raw) // M.CARD_DSL_WIDTH
    dsl_table = torch.from_numpy(dsl_raw.reshape(n_cards, M.CARD_DSL_WIDTH)).to(device)

    abil_raw = np.fromfile(data_dir / "card_abilities.bin", dtype=np.float32)
    abil_table = torch.from_numpy(
        abil_raw.reshape(n_cards, MAX_ABILITIES, ABILITY_DSL_WIDTH)
    ).to(device)

    # Filter to non-zero cards (`cid >= 1` with non-zero DSL vector)
    valid_mask = dsl_table.abs().sum(dim=-1) > 0
    valid_indices = torch.where(valid_mask)[0]
    n_valid = int(valid_indices.numel())

    cfg = M.Config4(
        d_model=args.d_model,
        n_layers=args.layers,
        n_heads=args.heads,
        d_ff=args.d_model * 4,
    )
    net = M.PlayerNet4(cfg).to(device)
    pre = PhaseAPretrainer(net).to(device)
    opt = torch.optim.AdamW(pre.parameters(), lr=args.lr, weight_decay=1e-2)

    print(
        f"[pretrain_dsl4] device={device} bf16={use_bf16} valid_cards={n_valid}/{n_cards} "
        f"d_model={cfg.d_model} layers={cfg.n_layers} steps={args.steps}"
    )

    t0 = time.perf_counter()
    dsl_sym = M.symlog(dsl_table)
    abil_sym = M.symlog(abil_table)

    for step in range(1, args.steps + 1):
        opt.zero_grad(set_to_none=True)
        idx = valid_indices[torch.randint(0, n_valid, (args.batch_size,), device=device)]
        x_dsl = dsl_sym[idx]  # [B, 160]
        x_abil = abil_sym[idx]  # [B, 8, 64]

        # 1. Masked feature autoencoding + InfoNCE contrastive alignment
        feat_drop = torch.rand_like(x_dsl) < 0.30
        x_masked = x_dsl.masked_fill(feat_drop, 0.0)

        # 2. Multi-card set context reconstruction through trunk_layers
        B_set = max(args.batch_size // 4, 8)
        S_len = args.set_size
        set_idx = valid_indices[
            torch.randint(0, n_valid, (B_set, S_len), device=device)
        ]
        set_dsl = dsl_sym[set_idx]  # [B_set, S_len, 160]
        card_mask_drop = torch.rand((B_set, S_len, 1), device=device) < 0.25
        feat_mask_drop = (torch.rand_like(set_dsl) < 0.40) | card_mask_drop
        set_dsl_masked = set_dsl.masked_fill(feat_mask_drop, 0.0)

        with torch.autocast(
            device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16
        ):
            h_card = net.dsl_proj(x_masked) + net.token_type_emb.weight[2]
            recon_single = pre.recon_head(h_card)
            loss_recon_single = F.mse_loss(recon_single[feat_drop], x_dsl[feat_drop])

            # Contrastive alignment between card DSL embedding and ability sequence
            h_card_full = net.dsl_proj(x_dsl)
            abil_valid = x_abil.abs().sum(dim=-1) > 0  # [B, 8]
            has_any_abil = abil_valid.any(dim=-1)
            h_abil_all = pre.ability_enc(x_abil)  # [B, 8, d]
            w_abil = abil_valid.float().unsqueeze(-1)
            h_abil_pool = (h_abil_all * w_abil).sum(dim=1) / w_abil.sum(dim=1).clamp(min=1.0)

            if has_any_abil.sum() >= 4:
                zc = F.normalize(pre.card_contrast_proj(h_card_full[has_any_abil]), dim=-1)
                za = F.normalize(pre.abil_contrast_proj(h_abil_pool[has_any_abil]), dim=-1)
                logits_ca = (zc @ za.T) / 0.07
                labels = torch.arange(zc.shape[0], device=device)
                loss_contrast = 0.5 * (
                    F.cross_entropy(logits_ca, labels) + F.cross_entropy(logits_ca.T, labels)
                )
            else:
                loss_contrast = torch.tensor(0.0, device=device)

            # Set-Transformer trunk forward
            tok = net.dsl_proj(set_dsl_masked) + net.token_type_emb.weight[2]
            for layer in net.trunk_layers:
                tok = layer(tok)
            tok = net.trunk_norm(tok)
            recon_set = pre.recon_head(tok)
            loss_recon_set = F.mse_loss(recon_set[feat_mask_drop], set_dsl[feat_mask_drop])

            loss = loss_recon_single + loss_recon_set + 0.25 * loss_contrast

        loss.backward()
        torch.nn.utils.clip_grad_norm_(pre.parameters(), 1.0)
        opt.step()

        if step == 1 or step % 100 == 0 or step == args.steps:
            elapsed = time.perf_counter() - t0
            print(
                f"[pretrain_dsl4] step={step}/{args.steps} "
                f"loss={loss.item():.4f} (single_mse={loss_recon_single.item():.4f} "
                f"set_mse={loss_recon_set.item():.4f} contrast={loss_contrast.item():.4f}) "
                f"({elapsed:.1f}s)"
            )

    out_dir = Path(args.out).expanduser()
    out_dir.mkdir(parents=True, exist_ok=True)
    torch.save(
        {
            "config": cfg.to_dict(),
            "state_dict": net.state_dict(),
            "step": 0,
            "phase_a_steps": args.steps,
            "phase_a_loss": float(loss.detach().item()),
        },
        out_dir / "net.pt",
    )
    atomic_json(
        out_dir / "meta.json",
        {
            "encoder_version": 4,
            "phase": "A_dsl_pretrained",
            "d_model": cfg.d_model,
            "n_layers": cfg.n_layers,
            "n_heads": cfg.n_heads,
            "phase_a_steps": args.steps,
            "phase_a_loss": float(loss.detach().item()),
        },
    )
    print(f"[pretrain_dsl4] saved Phase A checkpoint to {out_dir / 'net.pt'}")


if __name__ == "__main__":
    main()
