"""Explicit probe-wide precision metadata; no CUDA context is initialized here."""
import torch


def snapshot(compile_enabled):
    warn_only = getattr(torch, 'is_deterministic_algorithms_warn_only_enabled', None)
    return dict(float32_matmul_precision=torch.get_float32_matmul_precision(),
                cuda_matmul_allow_tf32=bool(torch.backends.cuda.matmul.allow_tf32),
                cudnn_allow_tf32=bool(torch.backends.cudnn.allow_tf32),
                cudnn_deterministic=bool(torch.backends.cudnn.deterministic),
                cudnn_benchmark=bool(torch.backends.cudnn.benchmark),
                deterministic_algorithms=bool(torch.are_deterministic_algorithms_enabled()),
                deterministic_algorithms_warn_only=None if warn_only is None else bool(warn_only()),
                compile_enabled=bool(compile_enabled))


def configure(policy, compile_enabled):
    """Inherit preserves existing behavior; highest explicitly excludes TF32."""
    if policy not in ('inherit', 'highest'):
        raise ValueError('Unsupported float32 precision policy')
    if policy == 'highest':
        torch.set_float32_matmul_precision('highest')
        torch.backends.cuda.matmul.allow_tf32 = False
        torch.backends.cudnn.allow_tf32 = False
    return snapshot(compile_enabled)


def require_unchanged(expected, compile_enabled):
    actual = snapshot(compile_enabled)
    if actual != expected:
        raise RuntimeError('Probe-wide precision flags changed between variants')
    return actual
