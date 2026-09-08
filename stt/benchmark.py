"""Replay a supplied WAV at wall-clock speed through the actual caption worker.
Writes only synthetic/test transcripts when explicitly given an output path.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time
import unicodedata
import psutil


def normalized(text):
    return ''.join(c.lower() for c in unicodedata.normalize('NFC', text) if c.isalnum())


def cer(reference, text):
    a, b = normalized(reference), normalized(text)
    row = list(range(len(b)+1))
    for i, char in enumerate(a, 1):
        new = [i]
        for j, other in enumerate(b, 1):
            new.append(min(new[-1]+1, row[j]+1, row[j-1]+(char != other)))
        row = new
    return row[-1] / max(1, len(a))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('wav')
    parser.add_argument('--model', default='base')
    parser.add_argument('--provider', default='whisper')
    parser.add_argument('--accelerator', default='cpu')
    parser.add_argument('--language', default='ko')
    parser.add_argument('--output', required=True)
    parser.add_argument('--reference', default='')
    args = parser.parse_args()
    start = time.perf_counter()
    proc = subprocess.Popen([sys.executable, '-u', str(Path(__file__).with_name('worker.py'))],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8',
        creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    proc.stdin.write(json.dumps(dict(provider=args.provider, model=args.model, accelerator=args.accelerator, language=args.language, wav=args.wav, realtime=True))+'\n')
    proc.stdin.flush()
    events, peak, errors = [], [0], []
    def monitor():
        process = psutil.Process(proc.pid)
        while proc.poll() is None:
            try:
                family = [process, *process.children(recursive=True)]
                peak[0] = max(peak[0], sum(p.memory_info().rss for p in family))
            except psutil.Error: break
            time.sleep(.1)
    def stderr():
        with open(args.output+'.stderr.log','w',encoding='utf-8') as log:
            for line in proc.stderr:
                log.write(line); log.flush()
                errors.append(line[-500:])
                if len(errors)>20: errors.pop(0)
    threading.Thread(target=monitor,daemon=True).start()
    threading.Thread(target=stderr,daemon=True).start()
    watchdog = threading.Timer(900, proc.kill)
    watchdog.start()
    try:
        for line in proc.stdout:
            event=json.loads(line)
            event['elapsed_s']=round(time.perf_counter()-start,3)
            events.append(event)
            print(json.dumps(event,ensure_ascii=False),flush=True)
        proc.wait(timeout=10)
    finally:
        watchdog.cancel()
        if proc.poll() is None: proc.kill()
        proc.stdin.close()
    text=' '.join(e['text'] for e in events if e['type']=='final')
    result=dict(provider=args.provider,model=args.model,language=args.language,peak_rss_mb=round(peak[0]/1048576,1),
                elapsed_s=round(time.perf_counter()-start,2),text=text,events=events,exit_code=proc.returncode)
    if args.reference: result['normalized_cer']=round(cer(args.reference,text),4)
    if not text: result['diagnostics']=errors
    Path(args.output).write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')


if __name__=='__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    main()
