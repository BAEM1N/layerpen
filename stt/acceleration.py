"""Probe actual runtimes; device selection never claims unsupported acceleration."""
import os
from pathlib import Path
from providers import SttError


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
    import openvino_genai as genai
    from huggingface_hub import snapshot_download
    model = config.get('model') or 'base'
    root = Path(os.environ.get('ONPEN_MODEL_DIR', os.environ.get('LAYERPEN_MODEL_DIR', Path.home() / '.cache' / 'onpen')))
    if Path(model).is_dir():
        location = model
    elif model in ('tiny', 'base', 'small'):
        location = str(root / ('ov-whisper-' + model))
        required = ['openvino_encoder_model.xml', 'openvino_encoder_model.bin',
                    'openvino_decoder_model.xml', 'openvino_decoder_model.bin', 'generation_config.json']
        if not all((Path(location) / name).is_file() for name in required):
            snapshot_download('OpenVINO/whisper-' + model + '-fp16-ov', local_dir=location,
                              allow_patterns=['*.json', '*.xml', '*.bin', '*.txt'])
    else:
        raise SttError('OpenVINO preview supports tiny, base, small, or a local OpenVINO model folder. Use CPU for other Whisper models.')
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
