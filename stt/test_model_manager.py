"""Offline model setup tests. Fake tiny payloads never download model weights."""
import importlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch

import model_manager as models


class ModelSetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.config = {'provider': 'whisper', 'model': 'base', 'accelerator': 'cpu'}
        self.entry = models.catalog_entry('whisper', 'base', 'faster-whisper')
        self.payloads = {'config.json': b'{}', 'model.bin': b'complete weights',
                         'tokenizer.json': b'{}', 'vocabulary.txt': b'words'}
        self.entry['files'] = {name: len(data) for name, data in self.payloads.items()}
        self.entry['downloadBytes'] = sum(self.entry['files'].values())
        self.env = patch.dict(os.environ, {}, clear=True)
        self.home = patch('model_manager.Path.home', return_value=self.root)
        self.catalog = patch('model_manager.catalog_entry', return_value=self.entry)
        self.modules = patch('model_manager.module_spec', return_value=None)
        for patcher in (self.env, self.home, self.catalog, self.modules):
            patcher.start()
            self.addCleanup(patcher.stop)
        self.addCleanup(self.temp.cleanup)

    def write_model(self, folder, omit=()):
        folder.mkdir(parents=True, exist_ok=True)
        for name, data in self.payloads.items():
            if name not in omit:
                (folder / name).write_bytes(data)

    def test_offline_status_does_not_import_or_download_or_create_cache(self):
        with patch('builtins.__import__', side_effect=AssertionError('Status must not import runtimes')):
            result = models.status(self.config)
        self.assertFalse(result['ready'])
        self.assertFalse(result['runtimeReady'])
        self.assertIn('faster_whisper', result['missingModules'])
        self.assertFalse((self.root / '.cache').exists())

    def test_missing_tokenizer_and_truncated_weights_are_not_ready(self):
        target = self.root / '.cache' / 'pointory' / 'whisper-base'
        self.write_model(target, omit=('tokenizer.json',))
        self.assertFalse(models.status(self.config)['ready'])
        self.write_model(target)
        (target / 'model.bin').write_bytes(b'partial')
        self.assertFalse(models.status(self.config)['ready'])
        with self.assertRaises(models.ModelError) as error:
            models.require_model(self.config)
        self.assertEqual(error.exception.code, 'model_required')

    def test_openvino_legacy_cache_requires_tokenizer_and_detokenizer(self):
        files = {'config.json': b'{}', 'generation_config.json': b'{}'}
        for prefix in ('openvino_encoder_model', 'openvino_decoder_model', 'openvino_tokenizer', 'openvino_detokenizer'):
            files[prefix + '.xml'] = b'<model/>'
            files[prefix + '.bin'] = b'weights'
        self.entry.update(engine='openvino', repo='OpenVINO/whisper-base-fp16-ov',
                          files={name: len(data) for name, data in files.items()})
        self.payloads = files
        config = {**self.config, 'accelerator': 'openvino:GPU'}
        old = self.root / '.cache' / 'onpen' / 'ov-whisper-base'
        self.write_model(old, omit=('openvino_detokenizer.bin',))
        self.assertFalse(models.status(config)['ready'])
        self.write_model(old)
        self.assertEqual(models.require_model(config), str(old))

    def test_completed_weights_reused_even_without_runtime(self):
        target = self.root / '.cache' / 'pointory' / 'whisper-base'
        self.write_model(target)
        result = models.status(self.config)
        self.assertTrue(result['ready'])
        self.assertFalse(result['runtimeReady'])
        events = []
        models.download(self.config, lambda kind, **data: events.append((kind, data)))
        self.assertEqual(events[-1][0], 'model_ready')
        self.assertTrue(events[-1][1]['reused'])
        self.assertEqual(models.require_model(self.config), str(target))

    def test_complete_legacy_and_hf_cache_are_reused_in_place(self):
        for brand in ('onpen', 'layerpen'):
            with self.subTest(brand=brand):
                old = self.root / '.cache' / brand / 'whisper-base'
                self.write_model(old)
                self.assertEqual(models.require_model(self.config), str(old))
                for child in old.iterdir():
                    child.unlink()
        hf = self.root / '.cache' / 'huggingface' / 'hub'
        snapshot = hf / 'models--Systran--faster-whisper-base' / 'snapshots' / ('a' * 40)
        self.write_model(snapshot)
        self.assertEqual(models.require_model(self.config), str(snapshot))
        self.assertFalse((self.root / '.cache' / 'pointory').exists())

    def test_explicit_roots_and_legacy_environment_precedence(self):
        explicit = self.root / 'models'
        with patch.dict(os.environ, {'POINTORY_MODEL_DIR': str(explicit), 'ONPEN_MODEL_DIR': 'older'}), \
             patch('model_manager.Path.home', side_effect=AssertionError('Override must not search the home folder')):
            self.write_model(explicit / 'models--Systran--faster-whisper-base' / 'snapshots' / self.entry['revision'])
            result = models.status(self.config)
            self.assertTrue(result['ready'])
            self.assertTrue(result['path'].startswith(str(explicit)))

    def test_custom_local_folder_is_read_only_and_not_a_download_source(self):
        folder = self.root / 'custom'
        self.write_model(folder)
        with patch('model_manager.catalog_entry', return_value=None):
            config = {**self.config, 'model': str(folder)}
            self.assertTrue(models.status(config)['ready'])
            self.assertFalse(models.status(config)['managed'])
            (folder / 'tokenizer.json').unlink()
            with self.assertRaises(models.ModelError) as error:
                models.download(config, lambda *args, **kwargs: None)
            self.assertEqual(error.exception.code, 'model_incomplete')

    def test_unreviewed_repository_or_accelerator_is_rejected_before_network(self):
        with patch('model_manager.catalog_entry', return_value=None):
            for name in ('other/private-model', 'https://example.test/model', '../missing'):
                with self.subTest(name=name), self.assertRaises(models.ModelError) as error:
                    models.status({**self.config, 'model': name})
                self.assertEqual(error.exception.code, 'model_unsupported')
        with self.assertRaises(models.ModelError):
            models.status({**self.config, 'accelerator': 'openvino:unknown'})

    def test_download_failure_redacted_and_retry_resumes_same_directory(self):
        target = self.root / '.cache' / 'pointory' / 'whisper-base'
        calls = []
        def snapshot(repo, **kwargs):
            calls.append(kwargs)
            target.mkdir(parents=True, exist_ok=True)
            if len(calls) == 1:
                self.write_model(target, omit=('model.bin',))
                partial = target / '.cache' / 'huggingface' / 'download'
                partial.mkdir(parents=True)
                (partial / 'model.bin.incomplete').write_bytes(b'partial')
                raise RuntimeError('private-token-do-not-show')
            self.assertTrue((target / '.cache' / 'huggingface' / 'download' / 'model.bin.incomplete').exists())
            self.write_model(target)
        with patch('model_manager.runtime_status', return_value={'runtimeReady': True, 'missingModules': [], 'downloadReady': True}), \
             patch.dict(sys.modules, {'huggingface_hub': SimpleNamespace(snapshot_download=snapshot)}):
            with self.assertRaises(models.ModelError) as error:
                models.download(self.config, lambda *args, **kwargs: None)
            self.assertEqual(error.exception.code, 'download_failed')
            self.assertNotIn('private-token', str(error.exception))
            self.assertFalse(models.status(self.config)['ready'])
            events = []
            result = models.download(self.config, lambda kind, **data: events.append((kind, data)))
        self.assertTrue(result['ready'])
        self.assertEqual(calls[0]['local_dir'], calls[1]['local_dir'])
        self.assertEqual(calls[0]['revision'], self.entry['revision'])
        self.assertFalse(calls[0]['token'])
        self.assertIn('tokenizer.json', calls[0]['allow_patterns'])
        progress = [data for kind, data in events if kind == 'model_progress']
        self.assertTrue(all(data['completedBytes'] <= data['totalBytes'] for data in progress))
        self.assertEqual(events[-1][0], 'model_ready')

    def test_reported_success_without_complete_payload_is_rejected(self):
        with patch('model_manager.runtime_status', return_value={'runtimeReady': True, 'missingModules': [], 'downloadReady': True}), \
             patch.dict(sys.modules, {'huggingface_hub': SimpleNamespace(snapshot_download=Mock())}):
            with self.assertRaises(models.ModelError) as error:
                models.download(self.config, lambda *args, **kwargs: None)
        self.assertEqual(error.exception.code, 'model_incomplete')


class CatalogAndWorkerTests(unittest.TestCase):
    def test_allowlist_sizes_complete_tokenizers_and_validated_auto_selection(self):
        for model in ('tiny', 'base', 'small', 'medium', 'large-v3', 'turbo'):
            entry = models.model_spec({'provider': 'whisper', 'model': model})
            self.assertIn('tokenizer.json', entry['files'])
            self.assertGreater(entry['downloadBytes'], 70_000_000)
            self.assertRegex(entry['revision'], '^[a-f0-9]{40}$')
        ov = models.model_spec({'provider': 'whisper', 'resolvedAccelerator': 'openvino:GPU'})
        self.assertEqual(ov['engine'], 'openvino')
        self.assertIn('openvino_detokenizer.bin', ov['files'])
        for model in ('0.6B', '1.7B'):
            qwen = models.model_spec({'provider': 'qwen', 'model': model, 'resolvedAccelerator': 'cuda'})
            self.assertEqual(qwen['accelerator'], 'cpu')
            self.assertIn('chat_template.json', qwen['files'])

    def test_status_cli_runs_without_site_packages_or_network(self):
        with tempfile.TemporaryDirectory() as temp:
            env = {**os.environ, 'POINTORY_MODEL_DIR': temp}
            result = subprocess.run([sys.executable, '-S', str(Path(models.__file__))],
                input=json.dumps({'provider': 'whisper', 'model': 'tiny'}) + '\n',
                text=True, capture_output=True, timeout=5, env=env)
        self.assertEqual(result.returncode, 0)
        event = json.loads(result.stdout)
        self.assertEqual(event['type'], 'model_status')
        self.assertFalse(event['ready'])
        self.assertFalse(event['runtimeReady'])

    def test_custom_qwen_requires_all_safetensor_shards_and_local_tokenizer(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            for name in ('config.json', 'preprocessor_config.json', 'tokenizer_config.json', 'vocab.json'):
                (folder / name).write_text('{}')
            (folder / 'merges.txt').write_text('words')
            (folder / 'model.safetensors.index.json').write_text(json.dumps({'weight_map': {
                'a': 'first.safetensors', 'b': 'second.safetensors'}}))
            (folder / 'first.safetensors').write_bytes(b'weights')
            config = {'provider': 'qwen', 'model': str(folder)}
            self.assertFalse(models.status(config)['ready'])
            (folder / 'second.safetensors').write_bytes(b'weights')
            self.assertTrue(models.status(config)['ready'])
            (folder / 'tokenizer_config.json').unlink()
            self.assertFalse(models.status(config)['ready'])

    @unittest.skipUnless(importlib.util.find_spec('numpy'), 'Inference worker test uses optional numpy runtime')
    def test_whisper_receives_prepared_path_and_never_downloads_on_start(self):
        import worker
        constructor = Mock()
        with patch('acceleration.hardware', return_value=[{'id': 'cpu', 'providers': ['whisper']}]), \
             patch('model_manager.require_model', return_value='prepared/model') as required, \
             patch.dict(sys.modules, {'faster_whisper': SimpleNamespace(WhisperModel=constructor)}), \
             patch('worker.emit'), patch.dict(os.environ, {}, clear=True):
            worker.recognizer({'provider': 'whisper', 'model': 'tiny'})
        self.assertEqual(constructor.call_args.args[0], 'prepared/model')
        self.assertTrue(constructor.call_args.kwargs['local_files_only'])
        required.assert_called_once_with({'provider': 'whisper', 'model': 'tiny', 'accelerator': 'cpu'})

    @unittest.skipUnless(importlib.util.find_spec('numpy'), 'Inference worker test uses optional numpy runtime')
    def test_qwen_receives_local_path_without_remote_code_or_network(self):
        import worker
        constructor = Mock()
        torch = SimpleNamespace(set_num_threads=Mock(), float32='float32')
        with patch('acceleration.hardware', return_value=[{'id': 'cpu', 'providers': ['qwen']}]), \
             patch('model_manager.require_model', return_value='prepared/qwen'), \
             patch.dict(sys.modules, {'torch': torch, 'qwen_asr': SimpleNamespace(Qwen3ASRModel=SimpleNamespace(from_pretrained=constructor))}), \
             patch('worker.emit'), patch.dict(os.environ, {}, clear=True):
            worker.recognizer({'provider': 'qwen'})
            self.assertEqual(os.environ['HF_HUB_OFFLINE'], '1')
            self.assertEqual(os.environ['TRANSFORMERS_OFFLINE'], '1')
        self.assertEqual(constructor.call_args.args[0], 'prepared/qwen')
        self.assertTrue(constructor.call_args.kwargs['local_files_only'])
        self.assertFalse(constructor.call_args.kwargs['trust_remote_code'])

    @unittest.skipUnless(importlib.util.find_spec('numpy'), 'Inference worker test uses optional numpy runtime')
    def test_missing_model_does_not_initialize_engine_or_audio(self):
        import worker
        with patch('acceleration.hardware', return_value=[{'id': 'cpu', 'providers': ['whisper']}]), \
             patch('model_manager.require_model', side_effect=models.ModelError('model_required', 'Prepare model')), \
             patch('worker.Audio.start') as audio, patch('worker.emit'):
            with self.assertRaises(models.ModelError):
                worker.recognizer({'provider': 'whisper', 'model': 'base'})
        audio.assert_not_called()


if __name__ == '__main__':
    unittest.main()
