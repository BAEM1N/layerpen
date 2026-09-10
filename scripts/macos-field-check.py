#!/usr/bin/env python3
"""Prepare and operate an isolated, click-to-run Mac field verification kit."""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import sys
from urllib.parse import urljoin


def stamp():
    return dt.datetime.now().strftime('%Y%m%d-%H%M%S-%f')


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_suffix(path.suffix + '.tmp')
    pending.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    pending.replace(path)


def kit_path(kit, relative):
    target = (kit / relative).resolve()
    if not target.is_relative_to(kit.resolve()):
        raise ValueError('A kit path points outside the prepared folder')
    return target


def desktop_ready():
    if platform.system() != 'Darwin':
        return False
    import pwd
    return os.stat('/dev/console').st_uid == os.getuid() and pwd.getpwuid(os.getuid()).pw_name != 'root'


def require_desktop():
    if not desktop_ready():
        raise RuntimeError('Mac 바탕화면까지 로그인한 뒤 실행하세요. / Log in to the Mac desktop first. No app was launched.')
    active = subprocess.run(['pgrep', '-x', 'pointory'], capture_output=True, text=True, timeout=10)
    if active.returncode == 0:
        raise RuntimeError('실행 중인 Pointory를 앱 종료 버튼으로 닫은 뒤 다시 실행하세요. / Close Pointory before starting this check. Other apps were not stopped.')
    if active.returncode != 1:
        raise RuntimeError('Could not check for another Pointory instance: ' + active.stderr.strip())


def app_environment(kit, manifest, profile):
    env = dict(os.environ)
    for key in list(env):
        if key.startswith('POINTORY_VALIDATION_'):
            env.pop(key)
    env['POINTORY_DATA_DIR'] = str(profile)
    if manifest.get('stt_python'):
        env['POINTORY_STT_PYTHON'] = manifest['stt_python']
    if manifest.get('model_dir'):
        env['POINTORY_MODEL_DIR'] = manifest['model_dir']
    return env


def seed_profile(profile, exports):
    profile.mkdir(parents=True, exist_ok=True)
    exports.mkdir(parents=True, exist_ok=True)
    settings = profile / 'settings.json'
    if not settings.exists():
        write_json(settings, {'language': 'ko', 'text_font': 'Apple SD Gothic Neo',
                             'settings_font_size': 14, 'capture_layer_only': True,
                             'capture_dir': str(exports), 'global_shortcut_enabled': False})


def prepare(args):
    kit = args.kit.resolve()
    build = kit / 'builds' / stamp()
    build.mkdir(parents=True)
    sources = {'app': args.app, 'validation_app': args.validation_app}
    manifest = {'source_revision': args.source_revision, 'prepared_at': dt.datetime.now().isoformat(),
                'python': str(Path(sys.executable).absolute()),
                # Preserve the venv executable path: resolving its symlink would
                # select the base interpreter and lose the installed STT modules.
                'stt_python': str(args.stt_python.expanduser().absolute()) if args.stt_python else None,
                'model_dir': str(args.model_dir.expanduser().absolute()) if getattr(args, 'model_dir', None) else None}
    for key, source in sources.items():
        if not source or not (source / 'Contents/MacOS/pointory').is_file():
            raise ValueError('Missing ' + key + ' Pointory.app')
        folder = build / ('Pointory.app' if key == 'app' else 'Pointory-validation.app')
        shutil.copytree(source, folder, symlinks=True)
        manifest[key] = str(folder.relative_to(kit))
        manifest[key + '_sha256'] = hashlib.sha256((folder / 'Contents/MacOS/pointory').read_bytes()).hexdigest()
    if args.dmg:
        destination = build / args.dmg.name
        shutil.copy2(args.dmg, destination)
        manifest['dmg'] = str(destination.relative_to(kit))
        manifest['dmg_sha256'] = hashlib.sha256(destination.read_bytes()).hexdigest()
    shutil.copy2(__file__, kit / 'field-check.py')
    if args.guide:
        guide = args.guide.read_text(encoding='utf-8')
        guide = re.sub(r'\]\((?![a-zA-Z]+:|#)([^)]+)\)', lambda match: '](' + urljoin(
            'https://github.com/BAEM1N/pointory/blob/main/docs/MAC-FIELD-CHECK.ko.md', match[1]) + ')', guide)
        (kit / '현장 확인표.md').write_text(guide, encoding='utf-8')
    (kit / 'results').mkdir(exist_ok=True)
    write_json(kit / 'manifest.json', manifest)
    (kit / '먼저 읽기.txt').write_text(
        'Pointory 현장 확인 준비\n\n1. Mac 바탕화면에 로그인합니다.\n'
        '2. 다른 Pointory가 실행 중이면 앱의 종료 버튼으로 닫습니다.\n'
        '3. Run Basic Checks.command를 실행하면 기본 검사가 두 번 진행되고 자동 종료됩니다.\n'
        '4. Open Results.command에서 latest-summary.txt와 상세 보고서를 확인합니다.\n'
        '5. Launch Pointory.command로 일반 앱을 열고 현장 확인표.md의 실조작 항목을 확인합니다.\n\n'
        '개발용 ad hoc 서명 앱이며 Developer ID 공증은 없습니다. 기본 검사는 화면·마이크를 녹화하지 않습니다.\n'
        '수동 설정: profiles/manual, 기본 캡처: results/manual-captures\n'
        '앱이 실행 중이면 자동 검사 런처는 해당 앱을 강제 종료하지 않습니다.\n', encoding='utf-8')
    for name, mode in [('Launch Pointory.command', 'launch'), ('Run Basic Checks.command', 'basic'), ('Open Results.command', 'results')]:
        command = '\n'.join(['#!/bin/bash', 'cd -- "$(dirname -- "$0")" || exit 1',
                             shlex.quote(manifest['python']) + ' ./field-check.py ' + mode + ' --kit "$PWD"',
                             'code=$?', 'printf "\\nPress Enter to close this window... "', 'read -r reply', 'exit "$code"', ''])
        target = kit / name
        target.write_text(command, encoding='utf-8')
        target.chmod(0o755)
    print('Prepared:', kit)
    return 0


def launch(kit, manifest):
    require_desktop()
    profile = kit / 'profiles/manual'
    seed_profile(profile, kit / 'results/manual-captures')
    env = app_environment(kit, manifest, profile)
    log = kit / 'results' / ('manual-' + stamp() + '.log')
    executable = kit_path(kit, manifest['app']) / 'Contents/MacOS/pointory'
    with log.open('w', encoding='utf-8') as output:
        child = subprocess.Popen([str(executable)], env=env, stdout=output, stderr=subprocess.STDOUT,
                                 start_new_session=True, cwd=str(kit))
    print('Pointory 실행 / launched. PID:', child.pid)
    print('필기 레이어 캡처가 기본입니다. 화면 기록 및 마이크 기능은 직접 선택할 때 확인하세요.')
    print('Profile:', profile)
    return 0


def basic(kit, manifest):
    require_desktop()
    run = kit / 'results' / ('basic-' + stamp())
    run.mkdir(parents=True)
    profile = run / 'profile'
    seed_profile(profile, run / 'exports')
    executable = kit_path(kit, manifest['validation_app']) / 'Contents/MacOS/pointory'
    reports = []
    for number in [1, 2]:
        report_file = run / ('run-%d.json' % number)
        env = app_environment(kit, manifest, profile)
        env['POINTORY_VALIDATION_REPORT'] = str(report_file)
        if number == 2:
            env['POINTORY_VALIDATION_PREVIOUS_REPORT'] = str(run / 'run-1.json')
        with (run / ('run-%d.log' % number)).open('w', encoding='utf-8') as log:
            child = subprocess.Popen([str(executable)], env=env, stdout=log, stderr=subprocess.STDOUT,
                                     start_new_session=True, cwd=str(kit))
            try:
                code = child.wait(timeout=240)
            except subprocess.TimeoutExpired:
                child.terminate()
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=10)
                code = -1
        try:
            report = json.loads(report_file.read_text(encoding='utf-8'))
        except (OSError, ValueError) as error:
            report = {'status': 'failed', 'errors': ['No usable report: ' + str(error)]}
        reports.append({'run': number, 'exit_code': code, 'status': report.get('status'),
                        'checks': report.get('checks', []), 'errors': report.get('errors', [])})
        if code != 0 or report.get('status') != 'passed':
            break
    passed = len(reports) == 2 and all(r['exit_code'] == 0 and r['status'] == 'passed' for r in reports)
    summary = {'status': 'passed' if passed else 'failed', 'source_revision': manifest['source_revision'],
               'scope': 'Isolated basic UI checks, ink-only exports, localhost server. No screen/microphone recording.',
               'reports': reports}
    write_json(run / 'summary.json', summary)
    write_json(kit / 'results/latest-basic.json', summary)
    lines = ['Pointory 기본 검사: ' + summary['status'].upper(), 'Source: ' + manifest['source_revision'], str(run), '']
    for report in reports:
        lines.append('Run %d: %s (exit %s)' % (report['run'], report['status'], report['exit_code']))
        for check in report['checks']:
            lines.append('  %s: %s %s' % (check.get('status'), check.get('name'), check.get('error', '')))
        lines.extend(str(error) for error in report['errors'])
    lines += ['', '실제 키보드/IME, 마우스 클릭 통과, 다른 기기 LAN, 권한 승인 기능은 현장 확인표로 별도 확인하세요.']
    (kit / 'results/latest-summary.txt').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print('\n'.join(lines))
    return 0 if passed else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['prepare', 'launch', 'basic', 'results'])
    parser.add_argument('--kit', type=Path, required=True)
    parser.add_argument('--app', type=Path)
    parser.add_argument('--validation-app', type=Path)
    parser.add_argument('--dmg', type=Path)
    parser.add_argument('--guide', type=Path)
    parser.add_argument('--stt-python', type=Path)
    parser.add_argument('--model-dir', type=Path, help='Optional prepared Whisper model cache for the manual app')
    parser.add_argument('--source-revision', default='development')
    args = parser.parse_args()
    if args.mode == 'prepare':
        return prepare(args)
    kit = args.kit.resolve()
    manifest = json.loads((kit / 'manifest.json').read_text(encoding='utf-8'))
    if args.mode == 'launch':
        return launch(kit, manifest)
    if args.mode == 'basic':
        return basic(kit, manifest)
    subprocess.run(['open', str(kit / 'results')], check=True, timeout=15)
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
