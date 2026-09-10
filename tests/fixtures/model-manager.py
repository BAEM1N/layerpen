"""Native lifecycle fixture. It never downloads, installs, or opens audio.

Only runs with the opt-in models validation suite and an isolated profile/output
directory. Models select deterministic scenarios: tiny completes, base waits,
small sends oversized JSONL, medium closes stdout before waiting. Runtime qwen
waits for cancellation; other runtime engines complete without invoking pip.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def require_fixture():
    profile = Path(os.environ.get('POINTORY_DATA_DIR', ''))
    output = Path(os.environ.get('POINTORY_MODEL_FIXTURE_DIR', ''))
    if (os.environ.get('POINTORY_VALIDATION_SUITE') != 'models'
            or not profile.is_absolute() or not output.is_absolute()
            or profile.resolve() == output.resolve()):
        raise RuntimeError('This fixture requires the isolated native models suite.')
    output.mkdir(parents=True, exist_ok=True)
    return output


OUTPUT = require_fixture()
PID = os.getpid()


def record(role, request=None):
    request = request or {}
    value = {'pid': PID, 'parentPid': os.getppid(), 'role': role,
             'receivedKeys': sorted(request), 'apiKeyReceived': 'api_key' in request,
             'provider': request.get('provider'), 'model': request.get('model'),
             'command': request.get('command'), 'engine': request.get('engine'),
             'runtimeDir': request.get('runtimeDir')}
    (OUTPUT / f'pid-{PID}.json').write_text(json.dumps(value), encoding='utf-8')
    return value


def emit(kind, **values):
    print(json.dumps({'type': kind, 'fixturePid': PID, **values}), flush=True)


def spawn_descendant():
    options = {'stdin': subprocess.DEVNULL, 'stdout': subprocess.DEVNULL,
               'stderr': subprocess.DEVNULL}
    if os.name == 'nt':
        options['creationflags'] = subprocess.CREATE_NO_WINDOW
    return subprocess.Popen([sys.executable, __file__, '--descendant'], **options)


def wait_for_termination(phase, close_stdout=False):
    child = spawn_descendant()
    emit('model_progress', phase=phase, fixtureChildPid=child.pid,
         completedBytes=1024, totalBytes=4096, progressBasis='completed_files')
    if close_stdout:
        sys.stdout.flush()
        os.close(sys.stdout.fileno())
    time.sleep(90)


def main():
    if '--descendant' in sys.argv:
        record('descendant')
        time.sleep(90)
        return
    request = json.loads(sys.stdin.readline(65537))
    operation = request.get('command')
    metadata = record(operation or ('runtime' if 'runtimeDir' in request else 'caption'), request)
    if operation == 'devices':
        print(json.dumps({'devices': [{'id': 'fixture-mic', 'name': 'Fixture (no audio)', 'loopback': False}]}), flush=True)
        return
    if operation == 'hardware':
        print(json.dumps({'devices': [{'id': 'cpu', 'name': 'Fixture CPU', 'providers': ['whisper', 'qwen']}]}), flush=True)
        return
    if 'runtimeDir' in request:
        if request['engine'] == 'qwen':
            wait_for_termination('runtime_install')
            return
        emit('model_progress', phase='runtime_install')
        time.sleep(.2)
        emit('runtime_ready', engine=request['engine'], ready=True, runtimeReady=True,
             fixtureRequest=metadata, path=request['runtimeDir'])
        return
    if operation is None:
        # This simulates an active caption process to test mutual exclusion. No
        # audio library is imported and no provider/network connection is made.
        emit('status', text='Fixture captions; no audio')
        time.sleep(90)
        return
    model = request.get('model', 'base')
    info = {'provider': request.get('provider', 'whisper'), 'model': model,
            'engine': 'faster-whisper', 'accelerator': 'cpu', 'ready': False,
            'runtimeReady': True, 'downloadReady': True, 'managed': True,
            'downloadBytes': 4096, 'path': str(OUTPUT / 'fake-model'),
            'fixtureRequest': metadata}
    if operation == 'status':
        emit('model_status', **info)
        return
    if operation != 'download':
        raise RuntimeError('Unexpected fixture command')
    emit('model_status', **info)
    if model == 'tiny':
        emit('model_progress', phase='verifying', completedBytes=4096, totalBytes=4096)
        time.sleep(.2)
        info['ready'] = True
        emit('model_ready', **info)
    elif model == 'small':
        emit('model_progress', phase='preparing')
        print('x' * 34000, flush=True)
        time.sleep(90)
    else:
        wait_for_termination('fixture_stdout_closed' if model == 'medium' else 'downloading',
                             close_stdout=model == 'medium')


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    main()
