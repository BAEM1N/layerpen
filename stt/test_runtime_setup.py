import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
import runtime_setup


class RuntimeSetupTests(unittest.TestCase):
    def test_cancelled_venv_creation_repairs_pip_before_installing(self):
        with tempfile.TemporaryDirectory() as temporary:
            target=(Path(temporary)/'Pointory'/'stt-venv').resolve()
            python=target/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
            python.parent.mkdir(parents=True)
            python.touch()
            (target/'pyvenv.cfg').write_text('home = base')
            with patch('runtime_setup.run_step') as run, patch('runtime_setup.emit'), \
                 patch('runtime_setup.interpreter_version',return_value=(3,13)):
                runtime_setup.setup({'engine':'faster-whisper','runtimeDir':str(target)})
            self.assertEqual(run.call_args_list[0].args[0],[str(python),'-m','ensurepip','--upgrade'])
            self.assertEqual(run.call_args_list[1].args[0][0],str(python))
            self.assertEqual((target/'pyvenv.cfg').read_text(),'home = base')

    def test_stale_python_39_is_upgraded_without_clearing_existing_contents(self):
        with tempfile.TemporaryDirectory() as temporary:
            target=(Path(temporary)/'Pointory'/'stt-venv').resolve()
            python=target/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
            python.parent.mkdir(parents=True)
            python.touch()
            (target/'pyvenv.cfg').write_text('version = 3.9.6')
            (target/'keep.txt').write_text('preserve')
            base=Path(getattr(sys,'_base_executable',sys.executable)).resolve()
            with patch('runtime_setup.run_step') as run, patch('runtime_setup.emit'), \
                 patch('runtime_setup.interpreter_version',side_effect=[(3,9),(3,13)]):
                runtime_setup.setup({'engine':'faster-whisper','runtimeDir':str(target)})
            self.assertEqual(run.call_args_list[0].args[0],[str(base),'-m','venv','--upgrade',str(target)])
            self.assertEqual(run.call_args_list[1].args[0],[str(python),'-m','ensurepip','--upgrade'])
            self.assertEqual((target/'keep.txt').read_text(),'preserve')
            self.assertNotIn('--clear',run.call_args_list[0].args[0])

    def test_broken_partial_venv_is_repaired_and_checked_before_bootstrap(self):
        for config_present in (False,True):
            with self.subTest(config_present=config_present), tempfile.TemporaryDirectory() as temporary:
                target=(Path(temporary)/'Pointory'/'stt-venv').resolve()
                python=target/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
                python.parent.mkdir(parents=True)
                python.touch()
                if config_present:(target/'pyvenv.cfg').write_text('partial')
                with patch('runtime_setup.run_step') as run, patch('runtime_setup.emit'), \
                     patch('runtime_setup.interpreter_version',side_effect=[None,(3,13)]):
                    runtime_setup.setup({'engine':'faster-whisper','runtimeDir':str(target)})
                self.assertIn('--upgrade',run.call_args_list[0].args[0])
                self.assertEqual(run.call_args_list[1].args[0],[str(python),'-m','ensurepip','--upgrade'])

    def test_failed_repair_never_installs_into_unusable_interpreter(self):
        with tempfile.TemporaryDirectory() as temporary:
            target=(Path(temporary)/'Pointory'/'stt-venv').resolve()
            target.mkdir(parents=True)
            with patch('runtime_setup.run_step') as run, patch('runtime_setup.emit'), \
                 patch('runtime_setup.interpreter_version',return_value=None):
                with self.assertRaisesRegex(RuntimeError,'runtime_create'):
                    runtime_setup.setup({'engine':'faster-whisper','runtimeDir':str(target)})
            self.assertEqual(run.call_count,1)

    def test_version_probe_is_isolated_bounded_and_tolerates_invalid_runtime(self):
        for result,expected in [(SimpleNamespace(returncode=0,stdout='3.13\n'),(3,13)),
                                (SimpleNamespace(returncode=1,stdout='3.13\n'),None),
                                (SimpleNamespace(returncode=0,stdout='invalid\n'),None)]:
            with self.subTest(result=result),patch('runtime_setup.subprocess.run',return_value=result) as run:
                self.assertEqual(runtime_setup.interpreter_version(Path('python')),expected)
            self.assertEqual(run.call_args.args[0][1:4],['-I','-S','-B'])
            self.assertEqual(run.call_args.kwargs['timeout'],2)
        for error in (OSError(),subprocess.TimeoutExpired('python',2)):
            with patch('runtime_setup.subprocess.run',side_effect=error):
                self.assertIsNone(runtime_setup.interpreter_version(Path('python')))

    @unittest.skipIf(os.name=='nt','POSIX launcher behavior')
    def test_posix_upgrade_replaces_stale_aliases_without_following_them(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve()
            target=root/'Pointory'/'stt-venv'
            directory=target/'bin'
            directory.mkdir(parents=True)
            old=root/'old-python'
            old.write_text('old interpreter stays intact')
            base=root/'python3.13'
            base.write_text('new interpreter')
            for name in ('python','python3','python3.13'):(directory/name).symlink_to(old)
            (directory/'keep').write_text('preserve')
            runtime_setup.repair_posix_launchers(target,base)
            for name in ('python','python3','python3.13'):
                self.assertEqual((directory/name).resolve(),base)
            self.assertEqual(old.read_text(),'old interpreter stays intact')
            self.assertEqual((directory/'keep').read_text(),'preserve')

    @unittest.skipIf(os.name=='nt','POSIX launcher behavior')
    def test_posix_upgrade_rejects_redirected_launcher_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve()
            target=root/'Pointory'/'stt-venv'
            target.mkdir(parents=True)
            external=root/'outside'
            external.mkdir()
            (target/'bin').symlink_to(external,target_is_directory=True)
            with self.assertRaisesRegex(ValueError,'invalid_runtime_path'):
                runtime_setup.repair_posix_launchers(target,root/'python3.13')
            self.assertEqual(list(external.iterdir()),[])

    def test_rejects_global_or_relative_target_without_starting_process(self):
        for target in ['.',str(Path(tempfile.gettempdir())/'system-python')]:
            with self.subTest(target=target), patch('runtime_setup.run_step') as run:
                with self.assertRaisesRegex(ValueError,'invalid_runtime_path'):
                    runtime_setup.setup({'engine':'faster-whisper','runtimeDir':target})
                run.assert_not_called()

    def test_unknown_engine_cannot_inject_pip_arguments(self):
        with patch('runtime_setup.run_step') as run:
            with self.assertRaisesRegex(ValueError,'invalid_engine'):
                runtime_setup.setup({'engine':'--index-url=https://example.invalid'})
            run.assert_not_called()


if __name__=='__main__':
    unittest.main()
