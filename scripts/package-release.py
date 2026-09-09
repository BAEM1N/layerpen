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
setup=target/'bundle/nsis'/f'Pointory_{version}_x64-setup.exe'
binary=target/'pointory.exe'
assert setup.is_file() and binary.is_file(), 'Build the Windows installer first.'
shutil.copy2(setup,out/setup.name)
resources={'LICENSE':root/'LICENSE','THIRD-PARTY-NOTICES.txt':root/'THIRD-PARTY-NOTICES.txt','third-party-sources.zip':root/'third-party-sources.zip'}
resources['LICENSE.tauri']=root/'src-tauri/windows/LICENSE.tauri'
for name in ['GETTING-STARTED.md','INSTALL.ko.md','USER-GUIDE.ko.md','README.ja.md','README.zh-CN.md','STT-SETUP.md']:
 resources[name]=root/'docs'/name
with ZipFile(out/f'Pointory-{version}-windows-x64-portable.zip','w',ZIP_DEFLATED) as z:
 z.write(binary,'Pointory/Pointory.exe')
 for name,path in resources.items():z.write(path,'Pointory/'+name)
 for path in (root/'stt').glob('*'):
  if path.suffix in {'.py','.txt','.ps1'} and not path.name.startswith(('test_','benchmark')):z.write(path,'Pointory/stt/'+path.name)
files=[out/setup.name,out/f'Pointory-{version}-windows-x64-portable.zip']
for f in files:
 if f.suffix=='.zip':
  with ZipFile(f) as z:assert z.testzip() is None
(out/'SHA256SUMS.txt').write_text(''.join(hashlib.sha256(f.read_bytes()).hexdigest()+'  '+f.name+'\n' for f in files),encoding='utf-8')
for f in files:print(f'{f.name}: {f.stat().st_size:,} bytes')
