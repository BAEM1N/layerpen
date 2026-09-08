$ErrorActionPreference = 'Stop'
$runtime = Join-Path $env:LOCALAPPDATA 'OnPen\stt-venv'
py -3.13 -m venv $runtime
if ($LASTEXITCODE -ne 0) { throw 'Install Python 3.13 with the Python launcher first.' }
& (Join-Path $runtime 'Scripts\python.exe') -m pip install -r (Join-Path $PSScriptRoot 'requirements.txt')
if ($LASTEXITCODE -ne 0) { throw 'STT dependency installation failed.' }
Write-Host 'Restart OnPen. Optional Intel acceleration: install openvino and openvino-genai in this environment.'
