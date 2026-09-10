import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import runtime_setup


class RuntimeSetupTests(unittest.TestCase):
    def test_cancelled_venv_creation_repairs_pip_before_installing(self):
        with tempfile.TemporaryDirectory() as temporary:
            target=Path(temporary)/'Pointory'/'stt-venv'
            python=target/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
            python.parent.mkdir(parents=True)
            python.touch()
            (target/'pyvenv.cfg').write_text('home = base')
            with patch('runtime_setup.run_step') as run, patch('runtime_setup.emit'):
                runtime_setup.setup({'engine':'faster-whisper','runtimeDir':str(target)})
            self.assertEqual(run.call_args_list[0].args[0],[str(python),'-m','ensurepip','--upgrade'])
            self.assertEqual(run.call_args_list[1].args[0][0],str(python))
            self.assertEqual((target/'pyvenv.cfg').read_text(),'home = base')

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
