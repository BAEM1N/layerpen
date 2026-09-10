"""Explicit, resumable local model preparation. Status never imports an inference runtime.

Catalog revisions and file sizes were checked against the publishers' Hugging Face
repositories on 2026-09-10. Only catalog files are fetched; remote Python code is
never downloaded or executed. Existing compatible caches are read in place.
"""
import importlib.util
import json
import os
import re
import sys
import threading
from pathlib import Path

from acceleration import environment_value
from providers import DEFAULT_MODELS, SttError

OUTPUT_LOCK = threading.Lock()


class ModelError(SttError):
    def __init__(self, code, message):
        super().__init__(message)
        self.code = code


# Size metadata is also used to distinguish completed weights from interrupted files.
_WHISPER = {
    'tiny': ('d90ca5fe260221311c53c58e660288d3deb8d356', 75538270, 2249),
    'base': ('ebe41f70d5b6dfa9166e2c581c45c9c0cfc57b66', 145217532, 2309),
    'small': ('536b0662742c02347bc0e980a01041f333bce120', 483546902, 2370),
    'medium': ('08e178d48790749d25932bbc082711ddcfdfbc4f', 1527906378, 2257),
    'large-v3': ('edaa852ec7e145841d8ffdb056a99866b5f0a478', 3087284237, 2394),
    'turbo': ('0a363e9161cbc7ed1431c9597a8ceaf0c4f78fcf', 1617884929, 2263),
}
_OPENVINO = {
    'tiny': ('44662d68573bd732e50fc295d1c1ec47e77f67df', 59104668, 342799, 16416874, 165493, 3742, 1320),
    'base': ('84fbe975a79a8c996fd32c036558f29e2db6670f', 104006812, 496004, 41181290, 241299, 3802, 1320),
    'small': ('2410d022171ca8a97343182f88eec8807a324db9', 307161756, 960207, 176308330, 469182, 3841, 1325),
}


def catalog_entry(provider, model, engine):
    if provider == 'whisper' and engine == 'faster-whisper' and model in _WHISPER:
        revision, weight_size, config_size = _WHISPER[model]
        repo = ('mobiuslabsgmbh/faster-whisper-large-v3-turbo' if model == 'turbo'
                else 'Systran/faster-whisper-' + model)
        files = {'config.json': config_size, 'model.bin': weight_size,
                 'tokenizer.json': 2203239, 'vocabulary.txt': 459861}
        if model in ('large-v3', 'turbo'):
            files.pop('vocabulary.txt')
            files.update({'vocabulary.json': 1068114, 'preprocessor_config.json': 340,
                          'tokenizer.json': 2480617 if model == 'large-v3' else 2710337})
        license_id = 'MIT'
    elif provider == 'whisper' and engine == 'openvino' and model in _OPENVINO:
        revision, dbin, dxml, ebin, exml, generation, config = _OPENVINO[model]
        repo = 'OpenVINO/whisper-' + model + '-fp16-ov'
        files = {'openvino_decoder_model.bin': dbin, 'openvino_decoder_model.xml': dxml,
                 'openvino_encoder_model.bin': ebin, 'openvino_encoder_model.xml': exml,
                 'openvino_detokenizer.bin': 736181, 'openvino_detokenizer.xml': 9699,
                 'openvino_tokenizer.bin': 1898933, 'openvino_tokenizer.xml': 27011,
                 'generation_config.json': generation, 'config.json': config,
                 'added_tokens.json': 34604, 'merges.txt': 493869, 'normalizer.json': 52666,
                 'preprocessor_config.json': 356, 'special_tokens_map.json': 2194,
                 'tokenizer.json': 3930494, 'tokenizer_config.json': 282713, 'vocab.json': 835550}
        license_id = 'Apache-2.0'
    elif provider == 'qwen' and model in ('Qwen/Qwen3-ASR-0.6B', 'Qwen/Qwen3-ASR-1.7B'):
        small = model.endswith('0.6B')
        repo = model
        revision = ('5eb144179a02acc5e5ba31e748d22b0cf3e303b0' if small
                    else '7278e1e70fe206f11671096ffdd38061171dd6e5')
        files = {'chat_template.json': 1161, 'config.json': 6193 if small else 6194,
                 'generation_config.json': 142, 'merges.txt': 1671853,
                 'preprocessor_config.json': 330, 'tokenizer_config.json': 12487, 'vocab.json': 2776833}
        files.update({'model.safetensors': 1876091704} if small else {
            'model-00001-of-00002.safetensors': 4220320824,
            'model-00002-of-00002.safetensors': 478200688,
            'model.safetensors.index.json': 64821})
        license_id = 'Apache-2.0'
    else:
        return None
    return {'provider': provider, 'model': model, 'engine': engine, 'repo': repo,
            'revision': revision, 'files': files, 'license': license_id,
            'licenseUrl': 'https://huggingface.co/' + repo + '/blob/' + revision + '/README.md',
            'sourceUrl': 'https://huggingface.co/' + repo, 'downloadBytes': sum(files.values())}


def resolved_accelerator(config):
    if config.get('provider') == 'qwen':
        return 'cpu'
    value = config.get('accelerator') or 'auto'
    if value == 'auto':
        # UI supplies a result of the actual hardware probe. No heavy import in status.
        value = config.get('resolvedAccelerator') or 'cpu'
    if value not in ('cpu', 'cuda') and not re.fullmatch(r'openvino:(?:CPU|GPU|NPU)(?:\.\d+)?', value):
        raise ModelError('accelerator_invalid', 'Choose CPU or an available supported accelerator.')
    return value


def model_spec(config):
    provider = config.get('provider')
    if provider not in ('whisper', 'qwen'):
        raise ModelError('provider_invalid', 'Choose a supported local speech provider.')
    value = config.get('model') or DEFAULT_MODELS[provider]
    if not isinstance(value, str) or len(value) > 4096:
        raise ModelError('model_invalid', 'Choose a supported model or an existing local model folder.')
    accelerator = resolved_accelerator(config)
    engine = 'qwen' if provider == 'qwen' else 'openvino' if accelerator.startswith('openvino:') else 'faster-whisper'
    aliases = {'large': 'large-v3', 'large-v3-turbo': 'turbo'} if provider == 'whisper' else {
        '0.6B': 'Qwen/Qwen3-ASR-0.6B', '1.7B': 'Qwen/Qwen3-ASR-1.7B'}
    model = aliases.get(value, value)
    entry = catalog_entry(provider, model, engine)
    if entry:
        return {**entry, 'accelerator': accelerator, 'managed': True}
    try:
        folder = Path(value).expanduser()
        if folder.is_dir():
            return {'provider': provider, 'model': value, 'engine': engine,
                    'accelerator': accelerator, 'managed': False, 'localPath': str(folder.resolve()),
                    'files': {}, 'repo': None, 'revision': None, 'downloadBytes': None,
                    'license': None, 'licenseUrl': None, 'sourceUrl': None}
    except (OSError, ValueError):
        pass
    raise ModelError('model_unsupported', 'Choose a listed model or an existing local model folder. OpenVINO supports tiny, base and small.')


def model_root():
    override = environment_value('MODEL_DIR')
    return Path(override).expanduser() if override else Path.home() / '.cache' / 'pointory'


def managed_location(spec):
    name = ('ov-whisper-' if spec['engine'] == 'openvino' else 'whisper-') + spec['model']
    if spec['provider'] == 'qwen':
        name = 'qwen-' + spec['model'].rsplit('-', 1)[-1].lower()
    return model_root() / name


def hf_cache_roots():
    explicit = environment_value('MODEL_DIR')
    if explicit:
        # Earlier Whisper installations used the same override as cache_dir.
        return [Path(explicit).expanduser(), Path(explicit).expanduser() / 'hub']
    if os.environ.get('HF_HUB_CACHE') or os.environ.get('HUGGINGFACE_HUB_CACHE'):
        primary = Path(os.environ.get('HF_HUB_CACHE') or os.environ['HUGGINGFACE_HUB_CACHE'])
    elif os.environ.get('HF_HOME'):
        primary = Path(os.environ['HF_HOME']) / 'hub'
    else:
        primary = Path(os.environ.get('XDG_CACHE_HOME') or Path.home() / '.cache') / 'huggingface' / 'hub'
    roots = [primary]
    for brand in ('pointory', 'onpen', 'layerpen'):
        root = Path.home() / '.cache' / brand
        roots.extend((root, root / 'hub'))
    return roots


def candidate_locations(spec):
    if not spec['managed']:
        return [Path(spec['localPath'])]
    target = managed_location(spec)
    result = [target]
    if not environment_value('MODEL_DIR'):
        result.extend(Path.home() / '.cache' / brand / target.name for brand in ('onpen', 'layerpen'))
    for root in hf_cache_roots():
        repository = root / ('models--' + spec['repo'].replace('/', '--'))
        snapshots = repository / 'snapshots'
        result.append(snapshots / spec['revision'])
        try:
            ref = (repository / 'refs' / 'main').read_text(encoding='utf-8').strip()
            if re.fullmatch('[a-f0-9]{40}', ref):
                result.append(snapshots / ref)
        except OSError:
            pass
        try:
            result.extend(p for p in snapshots.iterdir() if p.is_dir() and re.fullmatch('[a-f0-9]{40}', p.name))
        except OSError:
            pass
    return list(dict.fromkeys(result))


def _file_size(path):
    try:
        return path.stat().st_size if path.is_file() else 0
    except OSError:
        return 0


def _json_object(path):
    try:
        return json.loads(path.read_text(encoding='utf-8')) if _file_size(path) < 16 * 1024 * 1024 else None
    except (OSError, ValueError, UnicodeError):
        return None


def complete_location(path, spec, exact=False):
    if not isinstance(_json_object(path / 'config.json'), dict):
        return False
    if spec['managed']:
        for name, size in spec['files'].items():
            actual = _file_size(path / name)
            if not actual or ((exact or name.endswith(('.bin', '.safetensors'))) and actual != size):
                return False
        return True
    if spec['engine'] == 'faster-whisper':
        names = ['model.bin', 'tokenizer.json']
        if not any(_file_size(path / n) for n in ('vocabulary.txt', 'vocabulary.json')):
            return False
    elif spec['engine'] == 'openvino':
        names = ['generation_config.json', 'openvino_encoder_model.xml', 'openvino_encoder_model.bin',
                 'openvino_decoder_model.xml', 'openvino_decoder_model.bin',
                 'openvino_tokenizer.xml', 'openvino_tokenizer.bin',
                 'openvino_detokenizer.xml', 'openvino_detokenizer.bin']
    else:
        names = ['preprocessor_config.json', 'tokenizer_config.json', 'vocab.json', 'merges.txt']
        if _file_size(path / 'model.safetensors'):
            names.append('model.safetensors')
        else:
            index = _json_object(path / 'model.safetensors.index.json')
            weights = index.get('weight_map', {}) if isinstance(index, dict) else {}
            if not weights or any(not isinstance(n, str) or Path(n).name != n or not n.endswith('.safetensors') for n in weights.values()):
                return False
            names.extend(set(weights.values()))
    return all(_file_size(path / name) for name in names)


def module_spec(name):
    try:
        return importlib.util.find_spec(name)
    except (ImportError, ValueError, AttributeError):
        return None


def runtime_status(engine):
    names = ['numpy', 'soundcard', 'websockets', 'huggingface_hub']
    names += {'faster-whisper': ['faster_whisper', 'ctranslate2', 'onnxruntime', 'tokenizers', 'av'],
              'openvino': ['openvino', 'openvino_genai'], 'qwen': ['qwen_asr', 'torch', 'transformers']}[engine]
    missing = [name for name in names if module_spec(name) is None]
    if engine == 'faster-whisper' and 'faster_whisper' not in missing:
        package = module_spec('faster_whisper')
        directory = Path(package.origin).parent if package and package.origin else None
        if not directory or not any(_file_size(directory / 'assets' / name)
                                    for name in ('silero_vad_v6.onnx', 'silero_vad.onnx')):
            missing.append('faster_whisper_vad_assets')
    return {'runtimeReady': not missing, 'missingModules': missing,
            'downloadReady': module_spec('huggingface_hub') is not None}


def status(config):
    spec = model_spec(config)
    candidates = candidate_locations(spec)
    location = next((path for path in candidates if complete_location(path, spec)), None)
    ready = location is not None
    return {key: spec[key] for key in ('provider', 'model', 'engine', 'accelerator', 'managed',
                                      'downloadBytes', 'license', 'licenseUrl', 'sourceUrl')} | {
        'phase': 'ready' if ready else 'missing', 'ready': ready,
        'path': str(location or candidates[0]), 'revision': spec['revision'],
        **runtime_status(spec['engine'])}


def require_model(config):
    """Resolve prepared files without any network operation, before opening audio."""
    spec = model_spec(config)
    location = next((path for path in candidate_locations(spec) if complete_location(path, spec)), None)
    if location is None:
        raise ModelError('model_required', 'Download the selected local speech model in Settings before starting captions.')
    return str(location)


def emit(kind, **data):
    with OUTPUT_LOCK:
        print(json.dumps({'type': kind, **data}, ensure_ascii=False), flush=True)


def download(config, report=emit):
    spec = model_spec(config)
    initial = status(config)
    report('model_status', **initial)
    if initial['ready']:
        report('model_ready', **initial, reused=True)
        return initial
    if not spec['managed']:
        raise ModelError('model_incomplete', 'The selected local folder does not contain all required model files.')
    if not initial['downloadReady']:
        raise ModelError('runtime_required', 'Install the speech runtime before downloading a model.')
    target = managed_location(spec)
    target.mkdir(parents=True, exist_ok=True)
    os.environ.setdefault('HF_HUB_DISABLE_IMPLICIT_TOKEN', '1')
    os.environ.setdefault('HF_HUB_DISABLE_PROGRESS_BARS', '1')
    from huggingface_hub import snapshot_download
    stop = threading.Event()

    def progress(phase='downloading'):
        completed = [(name, size) for name, size in spec['files'].items() if _file_size(target / name) == size]
        # Only completed files count: Xet can preallocate partial files to their final size.
        report('model_progress', phase=phase, provider=spec['provider'], model=spec['model'],
               engine=spec['engine'], completedBytes=sum(size for _, size in completed),
               totalBytes=spec['downloadBytes'], completedFiles=len(completed),
               totalFiles=len(spec['files']), progressBasis='completed_files')

    def monitor():
        while not stop.wait(1):
            progress()

    progress('preparing')
    thread = threading.Thread(target=monitor, daemon=True)
    thread.start()
    try:
        # HF retains resumable .incomplete data; no user cache is deleted on failure/cancel.
        snapshot_download(spec['repo'], revision=spec['revision'], local_dir=str(target),
                          allow_patterns=[*spec['files'], 'README.md', 'LICENSE', 'LICENSE.txt'],
                          token=False, max_workers=3)
        stop.set()
        thread.join(timeout=2)
        progress('verifying')
        if not complete_location(target, spec, exact=True):
            raise ModelError('model_incomplete', 'The download is incomplete. Retry to resume the model download.')
        result = status(config)
        report('model_ready', **result, reused=False)
        return result
    except ModelError:
        raise
    except Exception:
        raise ModelError('download_failed', 'Model download failed. Check the connection and free disk space, then retry to resume.') from None
    finally:
        stop.set()
        thread.join(timeout=2)


def main():
    try:
        config = json.loads(sys.stdin.readline(65537))
        if not isinstance(config, dict):
            raise ValueError()
        command = config.get('command', 'status')
        if command == 'status':
            emit('model_status', **status(config))
        elif command == 'download':
            download(config)
        else:
            raise ModelError('command_invalid', 'Choose a supported model preparation action.')
    except ModelError as exc:
        emit('model_error', code=exc.code, text=str(exc))
        return 1
    except Exception:
        emit('model_error', code='model_failed', text='Model preparation failed. Check the selected model and local runtime.')
        return 1
    return 0


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    raise SystemExit(main())
