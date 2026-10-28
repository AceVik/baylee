"""Supervised trainer argument parsing and architecture dispatch."""
import argparse
import model4 as M


def parse_args(default_architecture="v4"):
    global M
    ap = argparse.ArgumentParser(description="Train BayleeAI v4/v5 via stateful TBPTT distillation")
    ap.add_argument("--data", required=True, help="Dataset directory produced by convert4")
    ap.add_argument("--bc-replay-data", default="", help="Opt-in BC-only half-lane mixture: primary data stable table half0 + replay all (same path: complementary half1)")
    ap.add_argument("--bc-epoch-batches", type=int, default=0, help="Opt-in fixed BC mixture epoch: cycle exhausted source schedules; must cover both complete first passes (0 preserves ordinary tails)")
    ap.add_argument("--validation-data", default="", help="Separate immutable validation dataset; reuses its cache while training packages change")
    ap.add_argument("--out", required=True, help="Output checkpoint directory")
    ap.add_argument("--init", default="", help="Optional initial checkpoint directory (e.g. Phase A)")
    ap.add_argument("--init-state", default="", help="Continue weights/optimizer from last.pt on a new package; reset recurrent lanes and sampler")
    ap.add_argument("--d-model", type=int, default=512)
    ap.add_argument("--layers", "--n-layers", dest="layers", type=int, default=8)
    ap.add_argument("--heads", "--n-heads", dest="heads", type=int, default=8)
    ap.add_argument("--d-ff", type=int, default=0, help="FFN width (0 = 4 * d_model)")
    ap.add_argument("--batch-size", type=int, default=4)
    ap.add_argument("--validation-batch-size", type=int, default=0, help="Validation lanes (0=training batch size); keep fixed when comparing training batch sizes")
    ap.add_argument("--unroll", "--unroll-len", dest="unroll", type=int, default=16)
    ap.add_argument("--thinking-steps", type=int, default=2)
    ap.add_argument(
        "--thinking-curriculum",
        action="store_true",
        help="Cycle thinking_steps K across (1, 2, 4, 8) during training (Phase B)",
    )
    ap.add_argument("--epochs", type=int, default=3)
    ap.add_argument("--max-steps", type=int, default=0, help="Stop after N optimizer steps if > 0")
    ap.add_argument("--max-shards", type=int, default=0, help="Limit shards loaded (0 = all)")
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--relational-lr", type=float, default=0., help="Opt-in v5 inner learning rate; zero preserves original optimizer groups")
    ap.add_argument("--relational-output-lr", type=float, default=0., help="Relational output learning rate; zero inherits --relational-lr")
    ap.add_argument("--relational-warmup-steps", type=int, default=0, help="Linear warmup of relational groups over global optimizer updates")
    ap.add_argument("--max-entities", type=int, default=M.MAX_ENTITIES)
    ap.add_argument("--num-workers", type=int, default=0)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--resume", default="", help="Resume an atomic last.pt checkpoint")
    ap.add_argument("--checkpoint-every", type=int, default=100)
    ap.add_argument("--validation-every", type=int, default=1000)
    ap.add_argument("--checkpoint-selection", choices=("policy", "combined"), default="policy",
                    help="Supervised snapshot criterion; match evaluation is still required for promotion")
    ap.add_argument("--validate-initial", action="store_true", help="Include the untouched initial policy in checkpoint selection")
    ap.add_argument("--activation-checkpointing", action="store_true")
    ap.add_argument("--offload-checkpoint-inputs", action="store_true", help="Offload CUDA checkpoint inputs to pinned CPU memory; requires activation checkpointing, preserves the full-unroll graph")
    ap.add_argument("--saved-activation-layers", type=int, default=0, help="Keep the last N trunk layers' activations instead of recomputing them; uses more VRAM")
    ap.add_argument("--async-checkpoint", action="store_true", help="Stage one immutable last.pt snapshot and write it in the background")
    ap.add_argument("--time-budget-seconds", type=float, default=0)
    ap.add_argument("--dropout", type=float, default=0.0)
    ap.add_argument("--reference-execution", action="store_true", help="Use the original synchronized execution for A/B checks")
    ap.add_argument("--fused-optimizer", action="store_true", help="Fuse CUDA AdamW kernels; benchmark before enabling")
    ap.add_argument("--board-chunk", type=int, default=1, help="Independent observation encodings per trunk call; recurrence remains sequential")
    ap.add_argument("--compile-regions", action="store_true", help="Compile repeated transformer/thinking blocks with TorchInductor")
    ap.add_argument("--compile-readouts", action="store_true", help="Also compile recurrent updates and action/value readouts; requires --compile-regions")
    ap.add_argument("--cache-thinking-kv", action="store_true", help="Reuse this decision's board attention projections across thinking steps")
    ap.add_argument("--prune-unused-policy-heads", action="store_true", help="Skip only policy heads that contribute nothing to this frame's loss")
    ap.add_argument("--decision-chunk", type=int, default=1, help="Group independent readouts after causal memory updates; requires grouped boards and dropout=0")
    ap.add_argument("--gpu-prefetch", type=int, choices=range(4), default=0, help="Future CUDA input batches on a separate copy stream (0=off, 1=double buffer)")
    ap.add_argument("--freeze-trunk", action="store_true", help="Freeze board encoder, transformer trunk, recurrent memory, and critic; train only action/policy heads")
    ap.add_argument("--architecture", choices=("v4", "v5"), default=default_architecture)
    ap.add_argument("--relational-steps", type=int, choices=range(5), default=2)
    ap.add_argument("--relational-revision", type=int, choices=(1, 2), default=2)
    from .supervised_options import add_options, configuration
    add_options(ap)
    ap.add_argument("--compact-thinking-rows",action="store_true",help="Opt-in thinking only for CPU-planned valid decision rows; requires deferred execution and dropout=0")
    args = ap.parse_args()
    args.supervised_loss = configuration(args, ap)
    if args.architecture == "v5":
        import model5 as M
    else:
        import model4 as M
    identity = ({"architecture": M.ARCHITECTURE, "model_family": "v5"}
                if args.architecture == "v5" else {})
    if sum(bool(v) for v in (args.init, args.init_state, args.resume)) > 1:
        ap.error("Choose one of --init, --init-state or --resume")
    if args.batch_size < 1 or args.validation_batch_size < 0:
        ap.error("Training batch size must be positive; validation batch size must be nonnegative")
    if args.bc_epoch_batches < 0 or (args.bc_epoch_batches and not args.bc_replay_data):
        ap.error("--bc-epoch-batches requires --bc-replay-data and nonnegative batches")
    if args.bc_replay_data and (not args.validation_data or args.batch_size%2 or args.batch_size<2 or args.init_state or args.max_shards):
        ap.error("--bc-replay-data requires separate validation, even B>=2, full shards and no --init-state")
    args.validation_batch_size = args.validation_batch_size or args.batch_size
    if args.compile_readouts and not args.compile_regions:
        ap.error("--compile-readouts requires --compile-regions")
    if not 0 <= args.saved_activation_layers <= args.layers:
        ap.error("--saved-activation-layers must be between zero and --layers")
    if args.saved_activation_layers and not args.activation_checkpointing:
        ap.error("--saved-activation-layers requires --activation-checkpointing")
    if args.offload_checkpoint_inputs and not args.activation_checkpointing:
        ap.error("--offload-checkpoint-inputs requires --activation-checkpointing")

    if args.decision_chunk > 1 and (args.reference_execution or args.board_chunk < 2 or args.dropout != 0):
        ap.error("--decision-chunk > 1 requires --board-chunk >= 2, dropout=0 and optimized execution")
    if args.compact_thinking_rows and (args.dropout != 0 or args.decision_chunk <= 1):
        ap.error("--compact-thinking-rows requires --decision-chunk >1 and dropout=0")
    if args.decision_chunk < 1:
        ap.error("--decision-chunk must be positive")

    return args, M, identity
