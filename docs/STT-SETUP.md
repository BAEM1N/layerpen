# Optional live captions / 선택 기능: 실시간 자막

Drawing needs no Python. Captions are experimental and require a separate Python environment. Audio starts only when Start is pressed. API keys are kept for the running session and are not saved. Cloud providers have not been tested with a real paid key.

## Windows
Install Python 3.13 (including the `py` launcher). In the installed or extracted app folder run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\stt\setup-windows.ps1
```

This prepares Python dependencies in `%LOCALAPPDATA%/Pointory/stt-venv`. An existing `%LOCALAPPDATA%/OnPen/stt-venv` or `%LOCALAPPDATA%/LayerPen/stt-venv` is reused in place when available; virtual environments are not moved. Restart Pointory, open CC, select microphone/system audio and a device, provider and accelerator. The first local model run downloads model weights.

Optional Intel GPU/NPU support: run that environment's Python with `-m pip install openvino openvino-genai`. Appropriate hardware/drivers are required. Qwen additionally needs `qwen-asr` and currently uses CPU. Whisper CUDA requires the CTranslate2-compatible NVIDIA runtime. Unsupported hardware is not treated as accelerated.

OpenVINO models default to `~/.cache/pointory/ov-whisper-<model>`. Complete model downloads in `~/.cache/onpen` or `~/.cache/layerpen` are reused without moving or downloading them again. Set `POINTORY_MODEL_DIR` to choose a model root explicitly; `ONPEN_MODEL_DIR` and `LAYERPEN_MODEL_DIR` remain accepted. An explicit root takes priority over default and legacy locations. This setting applies to the OpenVINO path, not every provider's cache.

## macOS / Linux (not runtime-verified)
Create a Python environment and install `stt/requirements.txt`. Set `POINTORY_STT_PYTHON` to its absolute interpreter path before launching. `POINTORY_STT_WORKER` optionally overrides the bundled worker. The corresponding `ONPEN_` and `LAYERPEN_` names remain supported as compatibility aliases. Mac GPU/ANE support is a v0.2 validation task; this release must not be described as Apple Silicon accelerated.

Microphone, system-audio routing and capture permissions vary by OS. The current system-audio path has only been exercised on Windows. CPU/GPU/NPU availability is probed at runtime. No audio or transcript is automatically saved.
