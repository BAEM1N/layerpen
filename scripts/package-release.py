"""Package an already-built Windows release. Run from any working directory."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
import argparse, hashlib, json, os, shutil, subprocess, sys

root=Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser()
parser.add_argument('--output',required=True)
args=parser.parse_args()
out=Path(args.output).resolve()
if out==root or root in out.parents: raise SystemExit('Release output must be outside the source tree.')
out.mkdir(parents=True,exist_ok=True)
version=json.loads((root/'package.json').read_text())['version']
target=Path(os.environ.get('CARGO_TARGET_DIR',str(root/'src-tauri/target'))).resolve()/'release'
setup=target/'bundle/nsis'/f'LayerPen_{version}_x64-setup.exe'
binary=target/'monitor-ink.exe'
assert setup.is_file() and binary.is_file(), 'Build the Windows installer first.'
shutil.copy2(setup,out/setup.name)
resources={'LICENSE':root/'LICENSE','THIRD-PARTY-NOTICES.txt':root/'THIRD-PARTY-NOTICES.txt','third-party-sources.zip':root/'third-party-sources.zip'}
for name in ['GETTING-STARTED.md','INSTALL.ko.md','USER-GUIDE.ko.md','README.ja.md','README.zh-CN.md']:
 resources[name]=root/'docs'/name
with ZipFile(out/f'LayerPen-{version}-windows-x64-portable.zip','w',ZIP_DEFLATED) as z:
 z.write(binary,'LayerPen/LayerPen.exe')
 for name,path in resources.items():z.write(path,'LayerPen/'+name)
source=out/f'LayerPen-{version}-source.zip'
subprocess.run([sys.executable,str(root/'scripts/package-source.py'),str(source)],check=True)
shutil.copy2(root/'third-party-sources.zip',out/'third-party-sources.zip')
for f in out.glob('*.zip'):
 with ZipFile(f) as z:assert z.testzip() is None
files=sorted(f for f in out.iterdir() if f.is_file() and f.name!='SHA256SUMS.txt')
(out/'SHA256SUMS.txt').write_text(''.join(hashlib.sha256(f.read_bytes()).hexdigest()+'  '+f.name+'\n' for f in files),encoding='utf-8')
for f in files:print(f'{f.name}: {f.stat().st_size:,} bytes')
