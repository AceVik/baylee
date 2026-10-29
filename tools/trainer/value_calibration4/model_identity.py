"""Strict calibration dispatch; no model upgrades or inferred V5 revision changes."""
import model4 as V4


def source_model(saved):
    from encoder_contract import validate_encoder
    values = saved['config']
    validate_encoder(saved.get('encoder_version'), model_version=saved.get('model_version'),
                     relational_revision=values.get('relational_revision', 1),
                     candidate_context_revision=values.get('candidate_context_revision', 1),
                     global_feat_width=values.get('global_feat_width', 80))
    version = saved.get('model_version')
    if version == V4.MODEL_VERSION:
        if saved.get('model_family', 'v4') != 'v4' or saved.get('architecture') is not None:
            raise ValueError('V4 checkpoint has incompatible model identity')
        return V4.PlayerNet4(V4.Config4(**saved['config']))
    if version != 5:
        raise ValueError('Incompatible source model_version')
    import model5 as V5
    if saved.get('model_family', 'v5') != 'v5':
        raise ValueError('V5 model family differs')
    values = dict(saved['config'])
    architecture = saved.get('architecture')
    if 'relational_revision' not in values:
        if architecture != V5.ARCHITECTURES[1]:
            raise ValueError('V5 architecture requires explicit matching relational revision')
        values['relational_revision'] = 1
    config = V5.Config5(**values)
    if architecture != V5.architecture_for_config(config):
        raise ValueError('V5 architecture and relational revision disagree')
    return V5.PlayerNet5(config)


def source_identity(saved):
    """Preserve the original config bytes/fields, including old V1 omissions."""
    return {key: saved[key] for key in ('model_version', 'config', 'encoder_version',
                                       'trajectory_version', 'sampling_version',
                                       'architecture', 'model_family', 'structure_sha256', 'dsl_version') if key in saved}
