"""Package distributable source without local build/test artifacts."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
import sys

root = Path(__file__).resolve().parent.parent
dest = Path(sys.argv[1]).resolve()
if dest == root or root in dest.parents:
    raise SystemExit('Choose an output path outside the source repository.')
if dest.exists():
    raise SystemExit('Output already exists; choose a new filename.')
dest.parent.mkdir(parents=True, exist_ok=True)
excluded = {'.git', 'node_modules', 'target', 'gen', '__pycache__'}
with ZipFile(dest, 'w', ZIP_DEFLATED) as archive:
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root)
        if not path.is_file() or excluded.intersection(relative.parts):
            continue
        if relative.parts[:2] in {('video', 'out'), ('video', '.cache')}:
            continue
        if relative.parts[:2] == ('docs', 'validation') and path.name != '0.1.0.ko.md':
            continue
        if relative.parts[:2] == ('docs', 'research') or relative.parts[:2] == ('docs', 'assets'):
            continue
        if path.suffix.lower() in {'.exe', '.zip', '.log', '.pyc'} and path.name != 'third-party-sources.zip':
            continue
        archive.write(path, 'layerpen/' + relative.as_posix())
with ZipFile(dest) as archive:
    assert archive.testzip() is None
print(dest)
