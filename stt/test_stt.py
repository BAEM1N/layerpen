import unittest
import asyncio
import json
import io
import os
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch
import numpy as np
from providers import Protocol
from worker import Segmenter, Audio, STOP, cloud, main
from acceleration import choose, environment_value, openvino_recognizer


class RuntimeMigrationTests(unittest.TestCase):
    def test_model_directory_override_precedence_and_legacy_aliases(self):
        cases = [
            ({'POINTORY_MODEL_DIR': 'new', 'ONPEN_MODEL_DIR': 'old', 'LAYERPEN_MODEL_DIR': 'older'}, 'new'),
            ({'ONPEN_MODEL_DIR': 'old', 'LAYERPEN_MODEL_DIR': 'older'}, 'old'),
            ({'LAYERPEN_MODEL_DIR': 'older'}, 'older'),
            ({'POINTORY_MODEL_DIR': '', 'ONPEN_MODEL_DIR': 'old'}, 'old'),
            ({}, None),
        ]
        for values, expected in cases:
            with self.subTest(values=values), patch.dict(os.environ, values, clear=True):
                self.assertEqual(environment_value('MODEL_DIR'), expected)

    def test_diagnostics_setting_uses_pointory_and_preserves_legacy_flags(self):
        cases = [
            ({'POINTORY_STT_DIAGNOSTICS': '1'}, True),
            ({'POINTORY_STT_DIAGNOSTICS': '0', 'ONPEN_STT_DIAGNOSTICS': '1'}, False),
            ({'ONPEN_STT_DIAGNOSTICS': '1'}, True),
            ({'LAYERPEN_STT_DIAGNOSTICS': '1'}, True),
        ]
        for values, expected in cases:
            with self.subTest(values=values), patch.dict(os.environ, values, clear=True), \
                 patch('worker.sys.stdin', io.StringIO('{"command":"devices"}\n')), \
                 patch('worker.devices', return_value=[]), patch('worker.emit'), \
                 patch('faulthandler.dump_traceback_later') as diagnostics:
                main()
                self.assertEqual(diagnostics.called, expected)

    def test_openvino_reuses_prepared_legacy_location_without_download(self):
        for brand in ('onpen', 'layerpen'):
            with self.subTest(brand=brand), tempfile.TemporaryDirectory() as temp:
                home = Path(temp)
                legacy = home / '.cache' / brand / 'ov-whisper-base'
                download, pipeline = Mock(), Mock()
                with patch.dict(os.environ, {}, clear=True), patch('acceleration.Path.home', return_value=home), \
                     patch('model_manager.require_model', return_value=str(legacy)) as prepared, \
                     patch.dict('sys.modules', {'openvino_genai': SimpleNamespace(WhisperPipeline=pipeline),
                                               'huggingface_hub': SimpleNamespace(snapshot_download=download)}):
                    openvino_recognizer({}, 'CPU')
                download.assert_not_called()
                prepared.assert_called_once_with({'provider': 'whisper', 'accelerator': 'openvino:CPU'})
                pipeline.assert_called_once_with(str(legacy), 'CPU', CACHE_DIR=str(home / '.cache' / 'pointory' / 'ov-compiled-cache'))

    def test_openvino_missing_weights_requires_explicit_download(self):
        from model_manager import ModelError
        with tempfile.TemporaryDirectory() as temp:
            home = Path(temp)
            partial = home / '.cache' / 'onpen' / 'ov-whisper-base'
            partial.mkdir(parents=True)
            (partial / 'generation_config.json').write_text('{}')
            download, pipeline = Mock(), Mock()
            with patch.dict(os.environ, {}, clear=True), patch('acceleration.Path.home', return_value=home), \
                 patch.dict('sys.modules', {'openvino_genai': SimpleNamespace(WhisperPipeline=pipeline),
                                           'huggingface_hub': SimpleNamespace(snapshot_download=download)}):
                with self.assertRaises(ModelError) as error:
                    openvino_recognizer({}, 'CPU')
            self.assertEqual(error.exception.code, 'model_required')
            download.assert_not_called()
            pipeline.assert_not_called()

    def test_explicit_model_root_does_not_silently_download_or_search_default(self):
        from model_manager import ModelError
        with tempfile.TemporaryDirectory() as temp:
            custom = Path(temp) / 'custom'
            download, pipeline = Mock(), Mock()
            with patch.dict(os.environ, {'POINTORY_MODEL_DIR': str(custom)}, clear=True), \
                 patch('acceleration.Path.home', side_effect=AssertionError('Explicit cache must not search default folders')), \
                 patch.dict('sys.modules', {'openvino_genai': SimpleNamespace(WhisperPipeline=pipeline),
                                           'huggingface_hub': SimpleNamespace(snapshot_download=download)}):
                with self.assertRaises(ModelError) as error:
                    openvino_recognizer({}, 'CPU')
            self.assertEqual(error.exception.code, 'model_required')
            download.assert_not_called()
            pipeline.assert_not_called()


class AccelerationTests(unittest.TestCase):
    def test_auto_uses_supported_gpu_and_cpu_when_unavailable(self):
        devices = [{'id':'cpu','providers':['whisper','qwen']},{'id':'openvino:GPU','providers':['whisper']},{'id':'openvino:NPU','providers':['whisper']}]
        self.assertEqual(choose({'provider':'whisper'},devices),'openvino:GPU')
        self.assertEqual(choose({'provider':'qwen'},devices),'cpu')
        self.assertEqual(choose({'provider':'whisper'},devices[:1]),'cpu')
        self.assertEqual(choose({'provider':'whisper','accelerator':'openvino:NPU'},devices),'openvino:NPU')
        with self.assertRaises(RuntimeError): choose({'provider':'qwen','accelerator':'openvino:NPU'},devices)


class AudioDeviceTests(unittest.TestCase):
    def test_selected_coreaudio_id_survives_html_string_round_trip(self):
        # SoundCard's CoreAudio lookup requires integer keys. Other backends use strings.
        for platform, expected_id in [('darwin', 83), ('win32', '83'), ('linux', '83')]:
            with self.subTest(platform=platform):
                STOP.clear()
                recorder = Mock()
                def record(**kwargs):
                    STOP.set()
                    return np.zeros((1600, 1), np.float32)
                recorder.record.side_effect = record
                context = Mock()
                context.__enter__ = Mock(return_value=recorder)
                context.__exit__ = Mock(return_value=False)
                mic = SimpleNamespace(name='Selected input', isloopback=False,
                                      recorder=Mock(return_value=context))
                def get_microphone(device_id, **kwargs):
                    if type(device_id) is not type(expected_id) or device_id != expected_id:
                        raise IndexError('No device with that ID')
                    return mic
                backend = SimpleNamespace(get_microphone=Mock(side_effect=get_microphone))
                audio = Audio({'source': 'microphone', 'device': '83'}, 16000)
                try:
                    with patch('worker.sys.platform', platform), patch.dict('sys.modules', {'soundcard': backend}), \
                         patch('worker.emit') as emit:
                        audio.capture()
                    self.assertIsNone(audio.error)
                    backend.get_microphone.assert_called_once_with(expected_id, include_loopback=True)
                    recorder.record.assert_called_once_with(numframes=1600)
                    emit.assert_called_once_with('status', text='Listening', device='Selected input')
                finally:
                    STOP.clear()


class StreamingTests(unittest.TestCase):
    def test_no_hallucination_jobs_for_silence(self):
        s = Segmenter()
        for _ in range(1000):
            self.assertIsNone(s.feed(np.zeros(1600, np.float32)))
        self.assertEqual(len(s.blocks), 0)
        self.assertLessEqual(len(s.pre), 3)

    def test_partial_then_final_and_no_cross_segment_duplication(self):
        s = Segmenter()
        events = []
        for b in [np.ones(1600, np.float32)*.1]*20 + [np.zeros(1600, np.float32)]*8:
            result = s.feed(b)
            if result: events.append(result)
        self.assertFalse(events[0][1])
        self.assertTrue(events[-1][1])
        self.assertEqual(events[-1][2], 0)
        self.assertEqual(s.segment, 1)
        self.assertEqual(len(s.blocks), 0)

    def test_continuous_audio_bounded(self):
        s = Segmenter()
        for _ in range(10000):
            s.feed(np.ones(1600, np.float32)*.1)
            self.assertLess(len(s.blocks), 80)

    def test_backpressure_stops_instead_of_unbounded_latency(self):
        STOP.clear()
        audio = Audio({}, 16000)
        for _ in range(41): audio.put(np.zeros(1600, np.float32))
        self.assertTrue(STOP.is_set())
        self.assertIsNotNone(audio.error)
        self.assertEqual(audio.queue.qsize(), 40)
        STOP.clear()

    def test_openai_delta_final_and_late_old_turn(self):
        p = Protocol({'provider': 'openai'})
        p.receive({'type':'input_audio_buffer.committed','item_id':'a'})
        p.receive({'type':'input_audio_buffer.committed','item_id':'b'})
        kind = 'conversation.item.input_audio_transcription.'
        self.assertEqual(p.receive({'type':kind+'delta','item_id':'a','delta':'Hello'})[0]['text'], 'Hello')
        self.assertEqual(p.receive({'type':kind+'delta','item_id':'a','delta':' world'})[0]['text'], 'Hello world')
        self.assertEqual(p.receive({'type':kind+'completed','item_id':'b','transcript':'New'})[0]['type'], 'final')
        self.assertEqual(p.receive({'type':kind+'completed','item_id':'a','transcript':'Old'}), [])

    def test_gemini_interim_is_not_final(self):
        p = Protocol({'provider':'gemini'})
        self.assertEqual(p.receive({'serverContent':{'interimInputTranscription':{'text':'안녕'}}})[0]['type'], 'partial')
        self.assertEqual(p.receive({'serverContent':{'inputTranscription':{'text':'안녕하세요'}}})[0]['type'], 'final')

    def test_gemini_language_hints_use_supported_locale_codes(self):
        for language, expected in [('ko', ['ko-KR']), ('en', ['en-US']), ('ja', ['ja-JP']), ('zh', ['cmn-Hans-CN']), ('auto', [])]:
            p = Protocol({'provider': 'gemini', 'language': language})
            self.assertEqual(p.setup()['setup']['inputAudioTranscription']['languageCodes'], expected)

    def test_eleven_and_secret_redaction(self):
        p = Protocol({'provider':'elevenlabs','api_key':'secret'})
        self.assertEqual(p.receive({'message_type':'partial_transcript','text':'test'})[0]['type'], 'partial')
        with self.assertRaises(RuntimeError) as err:
            p.receive({'message_type':'error','error':'secret'})
        self.assertNotIn('secret', str(err.exception))

    def test_sample_rates_and_wire_formats(self):
        for provider, rate, field in [('openai',24000,'audio'),('gemini',16000,'realtimeInput'),('elevenlabs',16000,'audio_base_64')]:
            p = Protocol({'provider':provider, 'api_key':'test'})
            self.assertEqual(p.rate,rate)
            self.assertIn(field,p.audio(b'\0\0'))
            self.assertTrue(p.connection()[0].startswith('wss://'))


class CloudTransportTests(unittest.IsolatedAsyncioTestCase):
    async def test_all_three_stream_audio_and_deliver_captions(self):
        from websockets.asyncio.server import serve
        for provider in ('openai','gemini','elevenlabs'):
            with self.subTest(provider=provider):
                STOP.clear()
                events, chunks = [], []
                class TestAudio:
                    error = None
                    def __init__(self,*args): self.n=0
                    def start(self): pass
                    def next(self):
                        self.n+=1
                        if self.n>3:
                            import time
                            time.sleep(.2)
                            return None
                        return np.zeros(1600,np.float32)
                async def server(ws):
                    if provider!='elevenlabs':
                        setup=json.loads(await ws.recv())
                        self.assertIn('session' if provider=='openai' else 'setup',setup)
                    ready={'type':'session.updated'} if provider=='openai' else {'setupComplete':{}} if provider=='gemini' else {'message_type':'session_started'}
                    await ws.send(json.dumps(ready))
                    for _ in range(3): chunks.append(json.loads(await ws.recv()))
                    msg={'type':'conversation.item.input_audio_transcription.completed','item_id':'1','transcript':'test'} if provider=='openai' else {'serverContent':{'inputTranscription':{'text':'test'}}} if provider=='gemini' else {'message_type':'committed_transcript','text':'test'}
                    await ws.send(json.dumps(msg))
                    await ws.wait_closed()
                async with serve(server,'127.0.0.1',0) as srv:
                    port=srv.sockets[0].getsockname()[1]
                    with patch.object(Protocol,'connection',return_value=(f'ws://127.0.0.1:{port}',{})), patch('worker.Audio',TestAudio), patch('worker.emit',side_effect=lambda kind,**kw: events.append({'type':kind,**kw})):
                        await asyncio.wait_for(cloud({'provider':provider,'api_key':'fake-test-key'}),5)
                self.assertEqual(len(chunks),3)
                self.assertTrue(any(e.get('text')=='test' and e['type']=='final' for e in events))
        STOP.clear()

    async def test_auth_failure_never_opens_audio(self):
        from websockets.asyncio.server import serve
        async def server(ws):
            await ws.recv()
            await ws.send(json.dumps({'error':{'message':'secret'}}))
        STOP.clear()
        async with serve(server,'127.0.0.1',0) as srv:
            port=srv.sockets[0].getsockname()[1]
            with patch.object(Protocol,'connection',return_value=(f'ws://127.0.0.1:{port}',{})),patch.object(Audio,'start') as start:
                with self.assertRaises(RuntimeError): await cloud({'provider':'openai'})
                start.assert_not_called()


if __name__ == '__main__': unittest.main()
