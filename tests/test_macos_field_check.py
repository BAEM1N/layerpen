"""Headless regression checks for the onsite launcher, independent of a GUI."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('field_check', Path(__file__).parents[1] / 'scripts/macos-field-check.py')
field = importlib.util.module_from_spec(spec)
spec.loader.exec_module(field)


class FieldKitTests(unittest.TestCase):
    def test_logged_out_session_never_spawns_an_app(self):
        with patch.object(field, 'desktop_ready', return_value=False), patch.object(field.subprocess, 'Popen') as spawn:
            with self.assertRaisesRegex(RuntimeError, 'Log in'):
                field.basic(Path('/unused'), {})
            spawn.assert_not_called()

    def test_child_environment_cannot_inherit_capture_permission_probe(self):
        with patch.dict(field.os.environ, {'POINTORY_VALIDATION_SCREEN': '1', 'POINTORY_VALIDATION_KEEP_OPEN': '1'}):
            env = field.app_environment(Path('/kit'), {}, Path('/isolated'))
        self.assertNotIn('POINTORY_VALIDATION_SCREEN', env)
        self.assertNotIn('POINTORY_VALIDATION_KEEP_OPEN', env)
        self.assertEqual(env['POINTORY_DATA_DIR'], str(Path('/isolated')))

    def test_preparation_preserves_manual_settings_and_rewrites_guide_links(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            app = base / 'source/Pointory.app'
            binary = app / 'Contents/MacOS/pointory'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'fixture only')
            kit = base / 'kit'
            manual = kit / 'profiles/manual'
            field.seed_profile(manual, kit / 'results/manual-captures')
            settings = manual / 'settings.json'
            original = settings.read_bytes()
            guide = base / 'guide.md'
            guide.write_text('[fonts](../Wiki/KR/04-text-fonts.md)\n[section](#test)', encoding='utf-8')
            args = argparse.Namespace(kit=kit, app=app, validation_app=app, dmg=None, guide=guide, stt_python=None, source_revision='fixture')
            field.prepare(args)
            manifest = json.loads((kit / 'manifest.json').read_text(encoding='utf-8'))
            self.assertTrue(field.kit_path(kit, manifest['app']).is_dir())
            self.assertIn('https://github.com/BAEM1N/pointory/blob/main/Wiki/KR/04-text-fonts.md', (kit / '현장 확인표.md').read_text(encoding='utf-8'))
            self.assertEqual(settings.read_bytes(), original)
            self.assertTrue((kit / 'Run Basic Checks.command').is_file())

    def test_manifest_paths_cannot_escape_the_kit(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(ValueError):
                field.kit_path(Path(folder), '../another-app')


if __name__ == '__main__':
    unittest.main()
