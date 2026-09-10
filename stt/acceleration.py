"""Probe actual runtimes; device selection never claims unsupported acceleration."""
import os
from pathlib import Path
from providers import SttError


def environment_value(name):
    """Prefer Pointory settings while preserving earlier installation overrides."""
    return next((value for brand in ('POINTORY', 'ONPEN', 'LAYERPEN')
                 if (value := os.environ.get(brand + '_' + name))), None)


def hardware():
    result = [{'id': 'cpu', 'name': 'CPU', 'providers': ['whisper', 'qwen']}]
    try:
        import ctranslate2
        if ctranslate2.get_cuda_device_count():
            result.append({'id': 'cuda', 'name': 'NVIDIA GPU · CUDA', 'providers': ['whisper']})
    except Exception:
        pass
    try:
        import openvino as ov
        import openvino_genai
        core = ov.Core()
        for device in core.available_devices:
            if device.startswith(('GPU', 'NPU')):
                result.append({'id': 'openvino:' + device,
                               'name': core.get_property(device, 'FULL_DEVICE_NAME') + ' · OpenVINO',
                               'providers': ['whisper']})
    except Exception:
        pass
    return result


def choose(config, devices):
    requested = config.get('accelerator', 'auto')
    available = [d['id'] for d in devices if config['provider'] in d['providers']]
    if requested != 'auto':
        if requested not in available:
            raise SttError('Selected accelerator is unavailable for this provider. Refresh devices or choose CPU.')
        return requested
    for prefix in ('cuda', 'openvino:GPU', 'openvino:NPU', 'cpu'):
        match = next((d for d in available if d.startswith(prefix)), None)
        if match:
            return match
    return 'cpu'


def openvino_recognizer(config, device):
    from model_manager import require_model
    location = require_model({**config, 'provider': 'whisper', 'accelerator': 'openvino:' + device})
    import openvino_genai as genai
    root = Path(environment_value('MODEL_DIR') or Path.home() / '.cache' / 'pointory')
    cache = root / 'ov-compiled-cache'
    cache.mkdir(parents=True, exist_ok=True)
    engine = genai.WhisperPipeline(location, device, CACHE_DIR=str(cache))
    options = {'max_new_tokens': 128, 'task': 'transcribe'}
    language = config.get('language', 'auto')
    if language != 'auto':
        options['language'] = '<|' + language + '|>'
    def transcribe(audio):
        result = engine.generate(audio.tolist(), **options)
        return ''.join(result.texts).strip()
    return transcribe
