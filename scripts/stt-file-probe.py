#!/usr/bin/env python3
"""Exercise Pointory's real STT worker with an authored audio file, never a mic.

The worker runs its normal segmentation and Whisper CPU inference pipeline. Its
stdin stays open until it exits: closing stdin after the initial configuration
would send an implicit stop. File playback is paced to avoid overflowing the
worker's bounded live-audio queue. A first online run may download model weights;
--offline requires those weights to have already been cached.

This is a prerecorded synthetic-file test, not microphone, caption UI, model
accuracy, or GPU/ANE verification. Exit 0 means the checks in report.json passed;
1 means a worker/protocol check failed; 2 means invocation/preparation failed.
"""

import argparse
from collections import Counter
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import threading
import time
import wave


def timestamp():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def existing_file(value):
    path = Path(value).expanduser().absolute()
    if not path.is_file():
        raise argparse.ArgumentTypeError('File does not exist: ' + value)
    # Do not resolve the executable's symlink: a venv's python must retain its
    # venv path rather than become the base interpreter.
    return path


def bounded_timeout(value):
    try:
        seconds = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError('Timeout must be an integer') from error
    if not 1 <= seconds <= 300:
        raise argparse.ArgumentTypeError('Timeout must be between 1 and 300 seconds')
    return seconds


def parse_args():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''Mac example (paths stay within an isolated validation workspace):
  python3 scripts/stt-file-probe.py \\
    --python "$WORKSPACE/tools/stt-venv/bin/python" \\
    --worker "$WORKSPACE/source/stt/worker.py" \\
    --audio "$WORKSPACE/evidence/authored-lesson.wav" \\
    --model-dir "$WORKSPACE/models/file-probe" \\
    --output-dir "$WORKSPACE/evidence/stt-file-online"

Repeat with --offline and a different --output-dir to verify cached inference.
Supply an authored speech file; this script never records or synthesizes audio.
Logs include the file transcript. Do not use private or third-party recordings.
''')
    parser.add_argument('--python', required=True, type=existing_file,
                        help='Python executable containing the STT dependencies (preserves venv symlinks)')
    parser.add_argument('--worker', required=True, type=existing_file, help='Actual Pointory stt/worker.py')
    parser.add_argument('--audio', required=True, type=existing_file, help='Authored speech audio file; no microphone access')
    parser.add_argument('--model-dir', required=True, type=Path, help='Isolated model and Hugging Face cache directory')
    parser.add_argument('--output-dir', required=True, type=Path, help='New or empty evidence directory')
    parser.add_argument('--model', default='tiny.en', help='Whisper model name or local model path (default: tiny.en)')
    parser.add_argument('--offline', action='store_true', help='Forbid Hugging Face/Transformers network downloads')
    parser.add_argument('--timeout', default=300, type=bounded_timeout, help='Worker run timeout in seconds, 1-300 (default: 300)')
    return parser.parse_args()


def audio_info(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    info = {'name': path.name, 'bytes': path.stat().st_size, 'sha256': digest.hexdigest()}
    try:
        with wave.open(str(path), 'rb') as audio:
            info.update({'sample_rate_hz': audio.getframerate(), 'channels': audio.getnchannels(),
                         'duration_seconds': round(audio.getnframes() / audio.getframerate(), 3)})
    except (wave.Error, EOFError):
        info['duration_seconds'] = None
        info['metadata_note'] = 'Not a standard PCM WAV; decoding is delegated to the actual worker.'
    return info


def child_environment(model_dir, offline):
    environment = os.environ.copy()
    environment.update({
        'PYTHONUNBUFFERED': '1', 'PYTHONIOENCODING': 'utf-8', 'PYTHONDONTWRITEBYTECODE': '1',
        'POINTORY_MODEL_DIR': str(model_dir),
        'HF_HOME': str(model_dir / 'huggingface'),
        'HF_HUB_CACHE': str(model_dir / 'huggingface' / 'hub'),
        'HUGGINGFACE_HUB_CACHE': str(model_dir / 'huggingface' / 'hub'),
        'TRANSFORMERS_CACHE': str(model_dir / 'huggingface' / 'transformers'),
        'XDG_CACHE_HOME': str(model_dir / 'cache'),
        'HF_HUB_DISABLE_IMPLICIT_TOKEN': '1',
        'HF_HUB_DISABLE_TELEMETRY': '1',
    })
    for name in ('HF_HUB_OFFLINE', 'TRANSFORMERS_OFFLINE', 'HF_DATASETS_OFFLINE'):
        environment[name] = '1' if offline else '0'
    return environment


def stop_child(process):
    """Signal only the child this probe created, never other worker processes."""
    if process.poll() is not None:
        return
    try:
        process.stdin.write('stop\n')
        process.stdin.flush()
        process.wait(timeout=2)
        return
    except (BrokenPipeError, OSError, subprocess.TimeoutExpired):
        pass
    if process.poll() is not None:
        return
    try:
        process.terminate()
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        try:
            process.kill()
        except ProcessLookupError:
            return
        process.wait(timeout=3)


def summary(values):
    values = [value for value in values if isinstance(value, (int, float))
              and not isinstance(value, bool) and math.isfinite(value) and value >= 0]
    if not values:
        return {'count': 0, 'min': None, 'median': None, 'max': None, 'sum': None}
    return {'count': len(values), 'min': min(values), 'median': statistics.median(values),
            'max': max(values), 'sum': sum(values)}


def run(args):
    args.model_dir = args.model_dir.expanduser().absolute()
    args.output_dir = args.output_dir.expanduser().absolute()
    if args.output_dir.exists() and any(args.output_dir.iterdir()):
        raise ValueError('Output directory must be new or empty to preserve previous evidence')
    args.output_dir.mkdir(parents=True, exist_ok=True)
    args.model_dir.mkdir(parents=True, exist_ok=True)
    metadata = audio_info(args.audio)
    if not metadata['bytes']:
        raise ValueError('Audio file is empty')
    config = {'provider': 'whisper', 'accelerator': 'cpu', 'language': 'en',
              'model': args.model, 'wav': str(args.audio), 'realtime': True}
    events, protocol_errors, execution_errors = [], [], []
    started_at, started = timestamp(), time.monotonic()
    timed_out, interrupted = False, False
    process = None
    reader = None

    def collect(stream, raw_log, event_log):
        try:
            for line_number, line in enumerate(stream, 1):
                raw_log.write(line)
                raw_log.flush()
                elapsed_ms = round((time.monotonic() - started) * 1000)
                try:
                    event = json.loads(line)
                    if not isinstance(event, dict) or not isinstance(event.get('type'), str):
                        raise ValueError('Expected a JSON object with a string type')
                except (ValueError, json.JSONDecodeError) as error:
                    protocol_errors.append({'line': line_number, 'reason': str(error)})
                    continue
                events.append((elapsed_ms, event))
                event_log.write(json.dumps({'elapsed_ms': elapsed_ms, 'event': event}, ensure_ascii=False) + '\n')
                event_log.flush()
        except (OSError, UnicodeError) as error:
            protocol_errors.append({'reason': 'Could not read worker stdout: ' + str(error)})

    with (args.output_dir / 'worker.stdout.jsonl').open('w', encoding='utf-8') as stdout_log, \
            (args.output_dir / 'worker.stderr.log').open('w', encoding='utf-8') as stderr_log, \
            (args.output_dir / 'events.jsonl').open('w', encoding='utf-8') as event_log:
        try:
            process = subprocess.Popen(
                [str(args.python), '-u', str(args.worker)], stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=stderr_log, text=True, encoding='utf-8',
                cwd=str(args.worker.parent), env=child_environment(args.model_dir, args.offline),
                bufsize=1,
            )
            reader = threading.Thread(target=collect, args=(process.stdout, stdout_log, event_log), daemon=True)
            reader.start()
            process.stdin.write(json.dumps(config) + '\n')
            process.stdin.flush()
            # communicate(input=...) would close stdin and stop the worker.
            process.wait(timeout=args.timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            execution_errors.append('Worker exceeded the configured %s-second timeout' % args.timeout)
        except KeyboardInterrupt:
            interrupted = True
            execution_errors.append('Probe was interrupted')
        except OSError as error:
            execution_errors.append('Could not start or configure worker: ' + str(error))
        finally:
            if process is not None:
                stop_child(process)
                if process.stdin is not None:
                    try:
                        process.stdin.close()
                    except (BrokenPipeError, OSError):
                        pass
                if reader is not None:
                    reader.join(timeout=5)
                    if reader.is_alive():
                        execution_errors.append('Worker stdout did not close after child exit')
                process.stdout.close()

    elapsed_ms = round((time.monotonic() - started) * 1000)
    payloads = [event for _, event in events]
    counts = Counter(event['type'] for event in payloads)
    finals = [event for event in payloads if event['type'] == 'final'
              and isinstance(event.get('text'), str) and event['text'].strip()]
    accelerators = [event.get('device') for event in payloads if event['type'] == 'accelerator']
    worker_errors = [event for event in payloads if event['type'] == 'error']
    exit_code = process.returncode if process else None
    checks = {
        'worker_exited_zero': exit_code == 0,
        'within_timeout': not timed_out,
        'uninterrupted': not interrupted,
        'execution_without_errors': not execution_errors,
        'jsonl_protocol_valid': not protocol_errors,
        'worker_without_error_events': not worker_errors,
        'cpu_accelerator_confirmed': bool(accelerators) and all(device == 'cpu' for device in accelerators),
        'nonempty_final_transcript': bool(finals),
        'stopped_event_received': counts['stopped'] > 0,
        'final_inference_timings_present': bool(finals) and summary([event.get('inference_ms') for event in finals])['count'] == len(finals),
    }
    inference_events = [event for event in payloads if event['type'] in ('partial', 'final')]
    metrics = {
        'total_elapsed_ms': elapsed_ms,
        'model_ready_elapsed_ms': next((at for at, event in events if event['type'] == 'status'
                                       and event.get('text') == 'Local model ready'), None),
        'first_nonempty_final_elapsed_ms': next((at for at, event in events if event in finals), None),
        'all_inference_ms': summary([event.get('inference_ms') for event in inference_events]),
        'nonempty_final_inference_ms': summary([event.get('inference_ms') for event in finals]),
        'nonempty_final_lag_ms': summary([event.get('lag_ms') for event in finals]),
        'notes': 'Worker-reported inference_ms measures each actual transcribe call; lag_ms includes queue time. '
                 'Partials may reprocess overlapping speech. Total elapsed includes startup, model load/download, '
                 'paced file playback, and shutdown; it is not a standalone real-time factor or accuracy score.',
    }
    report = {
        'status': 'passed' if all(checks.values()) else 'failed',
        'scope': 'Authored prerecorded synthetic audio file through the real Pointory Whisper CPU worker',
        'not_verified': ['Microphone input or permission', 'Caption UI', 'GPU/ANE acceleration',
                         'Speech-recognition accuracy across voices, languages, or classrooms'],
        'started_at_utc': started_at, 'ended_at_utc': timestamp(),
        'platform': {'system': platform.system(), 'machine': platform.machine(), 'macos': platform.mac_ver()[0]},
        'settings': {'provider': 'whisper', 'accelerator': 'cpu', 'language': 'en',
                     'model': args.model, 'offline': args.offline, 'realtime_file_playback': True,
                     'timeout_seconds': args.timeout},
        'audio': metadata, 'worker_exit_code': exit_code, 'checks': checks,
        'event_counts': dict(counts), 'accelerators': accelerators,
        'final_transcript': ' '.join(event['text'].strip() for event in finals),
        'metrics': metrics, 'worker_errors': worker_errors,
        'execution_errors': execution_errors, 'protocol_errors': protocol_errors,
        'logs': ['worker.stdout.jsonl', 'worker.stderr.log', 'events.jsonl'],
    }
    report_path = args.output_dir / 'report.json'
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'status': report['status'], 'report': str(report_path), 'checks': checks,
                      'final_transcript': report['final_transcript'], 'metrics': metrics}, ensure_ascii=False, indent=2))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    try:
        raise SystemExit(run(parse_args()))
    except (OSError, ValueError) as error:
        print('STT file probe preparation failed: ' + str(error), file=sys.stderr)
        raise SystemExit(2)
