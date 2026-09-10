# Optional live captions / 선택 기능: 실시간 자막

Drawing works without Python. Captions use a separate, optional Python runtime.
The local setup order is **install the speech engine → download a model → start
captions**. Model preparation does not open the microphone or capture system
audio. Audio begins only when you press **Start**.

한국어 단계별 안내: [실시간 자막 사용법](../Wiki/KR/07-live-captions.md).
English guide: [Live captions](../Wiki/EN/07-live-captions.md).

## First local setup

1. Install **Python 3.10 or newer** for your operating system. Pointory's engine
   installer creates a virtual environment and installs packages; it does not
   install Python itself. For the managed installer, Python must be available
   as `python` on Windows or `python3` on Linux. macOS also checks standard
   Homebrew and Python.org installation paths. Explicit Python overrides
   use a manually managed environment; see [Custom Python environments](#custom-python-environments).
2. Open **Live captions** using the caption icon in the toolbar or the settings
   button. Choose **Local Whisper**, a model, and an accelerator. `base` is the
   default; `tiny` requires a smaller download.
3. If the engine is missing, select **Prepare runtime / 음성 엔진 설치**. It prepares
   the required packages in Pointory's managed environment, rather than global
   Python. Intel OpenVINO and Qwen selections need their additional packages.
   This control is unavailable while an explicit speech Python override is set.
4. Download the selected model. The panel reports the model source, license,
   approximate download size, destination, and preparation state. Downloading
   does not start transcription. Select an existing compatible local model
   folder by entering its path in the Model field if you already have one.
5. Once preparation is complete, choose the spoken language, microphone or
   system audio, and input device. Press **Start**, speak, and check the reported
   runtime device. The first OpenVINO start may spend time compiling the model
   locally. Stop captions before changing models or preparing another engine.

Auto chooses among detected supported accelerators, or CPU when none is
available. The resolved device is used for both model preparation and caption
startup. If GPU/NPU initialization fails, select CPU and prepare its model before
trying again; the app does not silently switch engines after that failure.

Cancel stops the active preparation process. Downloaded data is retained so a
retry can reuse completed files and resume transfers supported by the model
host. Progress counts completed files, so the indicator may stay still while a
large weights file is being transferred. A canceled or incomplete model is not
treated as ready. Closing the captions settings window stops audio but lets
preparation continue; reopen it to see progress. Cancel or quit Pointory to stop
the active preparation job.

## Model choices and disk use

These are approximate model file totals; the Python environment, engine
packages, download metadata, and compiled caches require additional space.

| Engine | Models | Approximate download |
| --- | --- | --- |
| Whisper CPU / CUDA | tiny, base, small | 78 MB / 148 MB / 486 MB |
| Whisper CPU / CUDA | medium, large-v3, large-v3-turbo | 1.53 GB / 3.09 GB / 1.62 GB |
| Intel OpenVINO | tiny, base, small | Shown for the selected model in the app |
| Qwen CPU (experimental) | Qwen3-ASR-0.6B, Qwen3-ASR-1.7B | 1.88 GB / 4.70 GB |

The app downloads listed models from pinned publisher revisions, including
their configuration, tokenizer, weights, and available license notices. It does
not execute Python code from model repositories or download arbitrary model
repository identifiers. Existing local folders must contain the files required
by the selected engine. API providers do not require a local model download.

## Windows runtime

The in-app installer uses `%LOCALAPPDATA%/Pointory/stt-venv`. Existing Pointory,
OnPen, or LayerPen environments can be discovered and reused in place; virtual
environments are not moved. The installer leaves completed work in place after
canceling, and can be retried.

For a manual setup, the included Windows script uses **Python 3.13 and the `py`
launcher**. Run it from the installed or extracted app folder:

```powershell
powershell -NoProfile -File .\stt\setup-windows.ps1
```

Restart Pointory afterward. For an existing environment, optional Intel
packages can also be installed with that environment's interpreter:
`python -m pip install openvino openvino-genai`. Qwen additionally requires
`qwen-asr`. Whisper CUDA needs compatible NVIDIA drivers and the runtime
required by CTranslate2. Engine installation does not install GPU drivers.

## macOS and Linux

Install Python 3.10+ before opening the engine installer, and make `python3`
available to Pointory. If you prefer an explicit interpreter path, prepare a
custom environment as described below. `POINTORY_STT_WORKER` optionally overrides
the bundled worker; the corresponding `ONPEN_` and `LAYERPEN_` names remain
compatibility aliases.

On macOS, Pointory checks compatible Python versions on its PATH and in standard
Homebrew/Python.org locations, because Finder launches may have a different PATH
from Terminal. A system Python older than 3.10 is skipped. The managed speech
environment is `~/Library/Application Support/Pointory/stt-venv`; once prepared,
it is discovered before the system interpreter. Worker processes avoid writing
Python bytecode into the signed app bundle.

Microphone permissions and system-audio routing differ by OS. System audio has
only been exercised on Windows. Apple Metal/Core ML/ANE acceleration is not
implemented in this release. A detected device or installed package does not
by itself prove that driver initialization or model inference will succeed.

## Custom Python environments

Set `POINTORY_STT_PYTHON` to the absolute interpreter path of an environment you
manage yourself. `ONPEN_STT_PYTHON` and `LAYERPEN_STT_PYTHON` remain supported
compatibility aliases. With any explicit override set, **Prepare runtime**
reports `runtime_override` and does not install packages into that environment
or create a different managed environment.

Install `stt/requirements.txt` with the configured interpreter yourself, plus
`openvino openvino-genai` for Intel acceleration or `qwen-asr` for Qwen when
needed. Model status, download, and captions then use that interpreter. To use
Pointory's managed installer instead, remove all speech Python override
variables, make the platform's default Python available, restart Pointory, and
select **Prepare runtime**.

## Model storage

New downloads use `~/.cache/pointory/`, for example `whisper-base`,
`ov-whisper-base`, and `qwen-0.6b`. Complete compatible models already in Hugging
Face, OnPen, or LayerPen caches are reused in place when available. Set
`POINTORY_MODEL_DIR` to choose a model root. `ONPEN_MODEL_DIR` and
`LAYERPEN_MODEL_DIR` remain accepted aliases; an explicit root takes priority
over the default and legacy locations. OpenVINO also writes a local compiled
model cache when first starting transcription.

## API providers and privacy

Cloud providers use an API key and a model available to your account; there is
no local model download. The Python worker dependencies are still required.
If they are missing, the API provider screen shows **Speech runtime setup /
음성 엔진 준비**. Use **Prepare runtime** there to install the worker packages.
You can keep the API provider selected; no local model is downloaded.
For an explicit Python override, install the worker dependencies in that custom
environment as described above.
Keys are used for the running session and are not saved with preferences.
Pressing Start sends audio to the selected provider and may incur its usage
charges. Cloud transcription has not been verified with a real paid key in this
release. Pointory does not automatically save audio recordings or transcripts.
