$ErrorActionPreference = 'Stop'
$runtime = Join-Path $env:LOCALAPPDATA 'Pointory\stt-venv'
if (-not (Test-Path -LiteralPath (Join-Path $runtime 'Scripts\python.exe'))) {
    # Virtual environments contain absolute paths, so reuse legacy installs in place.
    foreach ($brand in @('OnPen', 'LayerPen')) {
        $legacyRuntime = Join-Path $env:LOCALAPPDATA "$brand\stt-venv"
        if (Test-Path -LiteralPath (Join-Path $legacyRuntime 'Scripts\python.exe')) {
            $runtime = $legacyRuntime
            break
        }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $runtime 'Scripts\python.exe'))) {
        py -3.13 -m venv $runtime
        if ($LASTEXITCODE -ne 0) { throw 'Install Python 3.13 with the Python launcher first.' }
    }
}
& (Join-Path $runtime 'Scripts\python.exe') -m pip install -r (Join-Path $PSScriptRoot 'requirements.txt')
if ($LASTEXITCODE -ne 0) { throw 'STT dependency installation failed.' }
Write-Host "Pointory caption runtime: $runtime"
Write-Host 'Restart Pointory. Optional Intel acceleration: install openvino and openvino-genai in this environment.'
