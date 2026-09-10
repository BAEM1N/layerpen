"""Local development caption worker. JSON config on stdin, bounded JSONL events on stdout.
Microphone/system audio only opens after explicit start. Stop/EOF releases it.
"""
import asyncio
import json
import os
import queue
import sys
import threading
import time
from collections import deque
from pathlib import Path

os.environ.setdefault('OPENBLAS_NUM_THREADS', '1')
os.environ.setdefault('OMP_NUM_THREADS', '4')
import numpy as np
from providers import DEFAULT_MODELS, Protocol, SttError
from acceleration import environment_value

STOP = threading.Event()
OUTPUT_LOCK = threading.Lock()


def emit(kind, **data):
    with OUTPUT_LOCK:
        print(json.dumps({'type': kind, **data}, ensure_ascii=False), flush=True)


def devices():
    import soundcard as sc
    return [{'id': d.id, 'name': d.name, 'loopback': d.isloopback}
            for d in sc.all_microphones(include_loopback=True)]


class Audio:
    def __init__(self, config, rate):
        self.config, self.rate = config, rate
        self.queue = queue.Queue(maxsize=40)  # Never accumulate more than 4 seconds.
        self.error = None
        self.thread = None

    def put(self, block):
        try:
            self.queue.put_nowait(block)
        except queue.Full:
            self.error = SttError('Audio processing is too slow. Choose a smaller model or a cloud provider.')
            STOP.set()

    def capture(self):
        try:
            if self.config.get('wav'):
                from faster_whisper.audio import decode_audio
                data = decode_audio(self.config['wav'], sampling_rate=self.rate)
                step = self.rate // 10
                for i in range(0, len(data) + self.rate, step):
                    if STOP.is_set():
                        break
                    block = data[i:i+step] if i < len(data) else np.zeros(step, np.float32)
                    self.put(block)
                    if self.config.get('realtime', True):
                        STOP.wait(.1)
                self.put(None)
                return
            import soundcard as sc
            source = self.config.get('source', 'microphone')
            device_id = self.config.get('device')
            # HTML option values are strings; CoreAudio identifies devices by integer.
            if sys.platform == 'darwin' and isinstance(device_id, str) and device_id.isdecimal():
                device_id = int(device_id)
            if device_id:
                mic = sc.get_microphone(device_id, include_loopback=True)
            elif source == 'system':
                mic = sc.get_microphone(sc.default_speaker().id, include_loopback=True)
            else:
                mic = sc.default_microphone()
            if mic is None or bool(mic.isloopback) != (source == 'system'):
                raise SttError('Select an available device matching the audio source.')
            # WASAPI single-channel capture is unreliable on some devices; capture all then mix.
            with mic.recorder(samplerate=self.rate, blocksize=self.rate//10) as recorder:
                emit('status', text='Listening', device=mic.name)
                while not STOP.is_set():
                    data = recorder.record(numframes=self.rate//10)
                    self.put(np.asarray(data.mean(axis=1), dtype=np.float32))
        except Exception:
            self.error = SttError('Audio input failed. Check microphone permission and the selected device.')
            STOP.set()

    def start(self):
        self.thread = threading.Thread(target=self.capture, daemon=True)
        self.thread.start()

    def next(self):
        while not STOP.is_set():
            try:
                return self.queue.get(timeout=.2)
            except queue.Empty:
                pass
        if self.error:
            raise self.error
        return None


class Segmenter:
    """Bounded 8-second speech windows; 300ms pre-roll, 600ms silence, 1.5s partials."""
    def __init__(self, threshold=.008):
        self.threshold = threshold
        self.pre = deque(maxlen=3)
        self.blocks = []
        self.silence = 0
        self.count = 0
        self.segment = 0

    def feed(self, block):
        voice = np.sqrt(np.mean(block * block)) >= self.threshold
        self.pre.append(block)
        if not self.blocks:
            if not voice:
                return None
            self.blocks = list(self.pre)
            self.pre.clear()
        else:
            self.blocks.append(block)
        self.silence = 0 if voice else self.silence + 1
        self.count += 1
        final = self.silence >= 6 or len(self.blocks) >= 80
        if final or self.count >= 15:
            result = (np.concatenate(self.blocks), final, self.segment)
            self.count = 0
            if final:
                self.segment += 1
                self.blocks = []
                self.pre.clear()
                self.silence = 0
            return result
        return None


def recognizer(config):
    language = config.get('language', 'auto')
    language = None if language == 'auto' else language
    emit('status', text='Loading downloaded local model')
    from acceleration import hardware, choose, openvino_recognizer
    from model_manager import require_model, ModelError
    selected = choose(config, hardware())
    prepared_config = {**config, 'accelerator': selected}
    if config['provider'] == 'whisper' and selected.startswith('openvino:'):
        try:
            engine = openvino_recognizer(prepared_config, selected.split(':', 1)[1])
            emit('accelerator', device=selected, text='Whisper · ' + selected)
            return engine
        except ModelError:
            # Missing files are an explicit setup step, never an implicit download.
            raise
        except Exception:
            if config.get('accelerator', 'auto') != 'auto':
                raise SttError('OpenVINO model/device initialization failed. Check the model and driver, or select CPU.')
            emit('status', text='Accelerator initialization failed; falling back to CPU')
            selected = 'cpu'
            prepared_config['accelerator'] = selected
    model = require_model(prepared_config)
    # Some processors do not forward local_files_only; keep their entire worker offline.
    os.environ['HF_HUB_OFFLINE'] = '1'
    os.environ['TRANSFORMERS_OFFLINE'] = '1'
    if config['provider'] == 'whisper':
        from faster_whisper import WhisperModel
        engine = WhisperModel(model, device=selected, compute_type='float16' if selected == 'cuda' else 'int8', cpu_threads=4,
                              local_files_only=True)
        emit('accelerator', device=selected, text='Whisper · ' + selected)
        def transcribe(audio):
            parts, _ = engine.transcribe(audio, language=language, beam_size=3,
                vad_filter=True, condition_on_previous_text=False)
            return ''.join(part.text for part in parts).strip()
        return transcribe
    import torch
    from qwen_asr import Qwen3ASRModel
    torch.set_num_threads(4)
    engine = Qwen3ASRModel.from_pretrained(model, dtype=torch.float32,
        device_map='cpu', max_inference_batch_size=1, max_new_tokens=128,
        local_files_only=True, trust_remote_code=False)
    emit('accelerator', device='cpu', text='Qwen3-ASR · CPU')
    names = {'ko': 'Korean', 'en': 'English', 'ja': 'Japanese', 'zh': 'Chinese'}
    return lambda audio: engine.transcribe(audio=(audio, 16000), language=names.get(language))[0].text


def local(config, transcribe):
    emit('status', text='Local model ready')
    audio = Audio(config, 16000)
    jobs = queue.Queue(maxsize=3)
    segmenter = Segmenter(float(config.get('threshold', .008)))
    errors = []

    def inference():
        try:
            while not STOP.is_set():
                job = jobs.get()
                if job is None:
                    return
                samples, final, segment, queued = job
                started = time.perf_counter()
                text = transcribe(samples)
                if not STOP.is_set():
                    emit('final' if final else 'partial', text=text, segment=str(segment),
                         inference_ms=round((time.perf_counter()-started)*1000),
                         lag_ms=round((time.perf_counter()-queued)*1000))
        except Exception as exc:
            errors.append(exc)
            STOP.set()

    task = threading.Thread(target=inference, daemon=True)
    task.start()
    audio.start()
    try:
        while not STOP.is_set():
            block = audio.next()
            if block is None:
                break
            job = segmenter.feed(block)
            if job:
                # Skip superseded partial work; never silently drop completed speech.
                if not job[1] and not jobs.empty():
                    continue
                try:
                    jobs.put_nowait((*job, time.perf_counter()))
                except queue.Full:
                    raise SttError('Local model cannot keep up with live audio. Try a smaller model or API.')
        if not STOP.is_set():
            if segmenter.blocks:
                jobs.put((np.concatenate(segmenter.blocks), True, segmenter.segment, time.perf_counter()), timeout=20)
            jobs.put(None, timeout=20)
            task.join(timeout=60)
            if task.is_alive():
                raise SttError('Local inference timed out.')
        if errors:
            raise SttError('Local inference failed. Verify the model and runtime dependencies.')
        if audio.error:
            raise audio.error
    finally:
        STOP.set()


async def cloud(config):
    from websockets.asyncio.client import connect
    protocol = Protocol(config)
    url, headers = protocol.connection()
    audio = Audio(config, protocol.rate)
    async with connect(url, additional_headers=headers, open_timeout=15, close_timeout=3, max_size=2**20) as ws:
        setup = protocol.setup()
        if setup:
            await ws.send(json.dumps(setup))
        # Do not open microphone until authentication and setup are accepted.
        while True:
            event = json.loads(await asyncio.wait_for(ws.recv(), 15))
            protocol.receive(event)
            if event.get('type') == 'session.updated' or 'setupComplete' in event or event.get('message_type') == 'session_started':
                break
        emit('status', text='Cloud connected; audio is sent to ' + protocol.provider)
        audio.start()

        async def send():
            while not STOP.is_set():
                block = await asyncio.to_thread(audio.next)
                if block is None:
                    break
                pcm = (np.clip(block, -1, 1) * 32767).astype('<i2').tobytes()
                await ws.send(json.dumps(protocol.audio(pcm)))
            if audio.error:
                raise audio.error
            # Let provider VAD finalize a finite WAV test before closing.
            if config.get('wav') and not STOP.is_set():
                await asyncio.sleep(3)

        async def receive():
            async for raw in ws:
                for event in protocol.receive(json.loads(raw)):
                    emit(event.pop('type'), **event)
            if not STOP.is_set():
                raise SttError('Provider disconnected. Restart captions to reconnect.')

        tasks = [asyncio.create_task(send()), asyncio.create_task(receive())]
        try:
            done, pending = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
            for task in done:
                task.result()
        finally:
            STOP.set()
            for task in tasks:
                task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)


def main():
    config = json.loads(sys.stdin.readline())
    if environment_value('STT_DIAGNOSTICS') == '1':
        import faulthandler
        faulthandler.dump_traceback_later(30, repeat=False)
    if config.get('command') == 'devices':
        emit('devices', devices=devices())
        return
    if config.get('command') == 'hardware':
        from acceleration import hardware
        emit('hardware', devices=hardware())
        return
    provider = config.get('provider')
    if provider not in DEFAULT_MODELS:
        raise ValueError('Choose a supported provider.')
    def control():
        for line in sys.stdin:
            if line.strip() == 'stop':
                break
        STOP.set()
    try:
        if provider in ('whisper', 'qwen'):
            # Initialize native math libraries before starting a blocking stdin thread.
            transcribe = recognizer(config)
            threading.Thread(target=control, daemon=True).start()
            local(config, transcribe)
        else:
            threading.Thread(target=control, daemon=True).start()
            asyncio.run(cloud(config))
    except Exception as exc:
        # Only our own messages are emitted: network/library exceptions can contain API keys.
        safe = str(exc) if isinstance(exc, SttError) else 'STT connection/model failed. Check network, dependencies, model access and API key.'
        emit('error', text=safe, code=getattr(exc, 'code', 'stt_failed'))
    finally:
        STOP.set()
        emit('stopped', text='Stopped')


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    main()
