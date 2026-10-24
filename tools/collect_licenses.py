"""Keep Cargo package attribution and supplied license texts in the portable bundle."""
import json,pathlib,subprocess
root=pathlib.Path(__file__).resolve().parents[1]
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1'],cwd=root))
lines=['Silbenlot Rust dependency notices','', 'All locked packages are listed, including build-only and other-platform packages.', 'Complete corresponding package sources are included in Silbenlot-Source.zip/vendor.', '']
for p in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
 if p['name']=='diktator':continue
 folder=pathlib.Path(p['manifest_path']).parent
 lines.extend(['='*72,f"{p['name']} {p['version']}",f"SPDX: {p.get('license') or 'See package source'}",f"Source: {p.get('repository') or p.get('homepage') or 'https://crates.io/crates/'+p['name']}",''])
 files=[f for f in folder.iterdir() if f.is_file() and f.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE'))]
 if p.get('license_file'):
  f=folder/p['license_file']
  if f.is_file() and f not in files:files.append(f)
 for f in sorted(files):lines.extend([f'--- {f.name} ---',f.read_text(errors='replace'),''])
(root/'licenses/Rust-dependencies.txt').write_text('\n'.join(lines))
print(len(metadata['packages'])-1,'package notices collected')
