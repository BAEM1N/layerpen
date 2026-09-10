"""Prepare optional STT packages in Pointory's managed venv, never global Python."""
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid

ENGINES = {
    'faster-whisper': [],
    'openvino': ['openvino>=2025.4,<2027', 'openvino-genai>=2025.4,<2027'],
    'qwen': ['qwen-asr>=0.0.6,<1'],
}


def emit(kind, **values):
    print(json.dumps({'type': kind, **values}), flush=True)


def run_step(args, phase):
    emit('model_progress', phase=phase, completedBytes=None, totalBytes=None)
    options = {'stdout': subprocess.DEVNULL, 'stderr': subprocess.DEVNULL}
    if os.name == 'nt':
        options['creationflags'] = subprocess.CREATE_NO_WINDOW
    # Children inherit the dedicated native job's process group for cancellation.
    result = subprocess.run(args, **options)
    if result.returncode:
        raise RuntimeError(phase)


def interpreter_version(python):
    """Probe only the interpreter, without user packages or network access."""
    options = {'stdin': subprocess.DEVNULL, 'stdout': subprocess.PIPE,
               'stderr': subprocess.DEVNULL, 'text': True, 'timeout': 2}
    if os.name == 'nt':
        options['creationflags'] = subprocess.CREATE_NO_WINDOW
    try:
        result = subprocess.run([str(python), '-I', '-S', '-B', '-c',
            "import sys; print('%d.%d' % sys.version_info[:2])"], **options)
        parts = result.stdout.strip().split('.')
        if result.returncode == 0 and len(parts) == 2 and all(part.isdecimal() for part in parts):
            return tuple(int(part) for part in parts)
    except (OSError, subprocess.SubprocessError, UnicodeError):
        pass
    return None


def repair_posix_launchers(target, base):
    """venv --upgrade keeps old aliases; replace only the managed Python launchers."""
    directory = target / 'bin'
    if directory.resolve() != directory:
        raise ValueError('invalid_runtime_path')
    directory.mkdir(parents=True, exist_ok=True)
    for name in dict.fromkeys(('python', 'python3', base.name)):
        temporary = directory / ('.pointory-python-' + uuid.uuid4().hex)
        try:
            temporary.symlink_to(base)
            temporary.replace(directory / name)
        finally:
            if temporary.is_symlink():
                temporary.unlink()


def setup(config):
    engine = config.get('engine')
    if engine not in ENGINES:
        raise ValueError('invalid_engine')
    if sys.version_info < (3, 10):
        raise ValueError('python_version')
    target = Path(config.get('runtimeDir', ''))
    # The native host provides this managed path; never clear or move an existing venv.
    if not target.is_absolute() or target.name != 'stt-venv' or target.parent.name != 'Pointory':
        raise ValueError('invalid_runtime_path')
    target = target.resolve()
    if target.name != 'stt-venv' or target.parent.name != 'Pointory':
        raise ValueError('invalid_runtime_path')
    python = target / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
    version = interpreter_version(python) if python.is_file() else None
    complete = (target / 'pyvenv.cfg').is_file() and version is not None and version >= (3, 10)
    if not complete:
        existing = target.exists()
        base = Path(getattr(sys, '_base_executable', sys.executable)).resolve()
        if existing and os.name != 'nt':
            repair_posix_launchers(target, base)
        run_step([str(base), '-m', 'venv', *(['--upgrade'] if existing else []), str(target)], 'runtime_create')
        repaired = interpreter_version(python)
        if repaired is None or repaired < (3, 10):
            raise RuntimeError('runtime_create')
    # Cancellation can leave an interpreter and pyvenv.cfg before pip is ready.
    run_step([str(python), '-m', 'ensurepip', '--upgrade'], 'runtime_bootstrap')
    requirements = Path(__file__).with_name('requirements.txt')
    if not requirements.is_file():
        raise ValueError('runtime_resources_missing')
    run_step([str(python), '-m', 'pip', '--isolated', 'install', '--disable-pip-version-check',
              '--no-input', '--index-url', 'https://pypi.org/simple', '-r', str(requirements),
              *ENGINES[engine]], 'runtime_install')
    checks = ['numpy', 'soundcard', 'websockets', 'faster_whisper', 'huggingface_hub']
    if engine == 'openvino':
        checks += ['openvino', 'openvino_genai']
    if engine == 'qwen':
        checks += ['qwen_asr', 'torch', 'transformers']
    run_step([str(python), '-c', ';'.join('import '+name for name in checks)], 'runtime_verify')
    emit('runtime_ready', engine=engine, path=str(target), ready=True, runtimeReady=True)


def main():
    try:
        config = json.loads(sys.stdin.readline())
        setup(config)
        return 0
    except ValueError as exc:
        code = str(exc) if str(exc) in {'invalid_engine', 'python_version', 'invalid_runtime_path', 'runtime_resources_missing'} else 'runtime_setup_failed'
    except Exception:
        code = 'runtime_setup_failed'
    emit('model_error', code=code, text='Could not prepare the speech runtime. Check Python, network and available disk space, then retry.')
    return 1


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    raise SystemExit(main())
