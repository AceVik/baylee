"""Thin Torch model configuration bridge to native semantic encoder admission."""
import json
from trajectory_store4 import _native
DEFAULT_ENCODER = 7
PAYMENT_ENCODER = 8


def validate_encoder(version, *, model_version=5, relational_revision=2):
    return json.loads(_native(['training-contract'],json.dumps(dict(kind='encoder',version=version,
        model_version=model_version,relational_revision=relational_revision),allow_nan=False).encode()))


def check_initial_encoder(source, target, explicit_from7=False):
    return json.loads(_native(['training-contract'],json.dumps(dict(kind='initial_encoder',source=source,
        target=target,explicit_from7=explicit_from7),allow_nan=False).encode()))
