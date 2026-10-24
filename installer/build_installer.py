#!/usr/bin/env python3
"""Package a build.py --layout-only payload as a single offline NSIS installer."""
import argparse,hashlib,json,os,pathlib,re,shutil,subprocess
ROOT=pathlib.Path(__file__).resolve().parents[1]
BASE=ROOT.parents[1]
VERSION=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M).group(1)
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def quote(s):return str(s).replace('$','$$').replace('"','$\\"')
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--cache',type=pathlib.Path,required=True);ap.add_argument('--out',type=pathlib.Path,required=True);ap.add_argument('--docker-container');ap.add_argument('--arch',choices=['both','x64','arm64'],default='both');args=ap.parse_args()
 cache=args.cache.resolve();out=args.out.resolve();out.mkdir(parents=True,exist_ok=True)
 def compiler_path(p):return '/workspace/'+p.relative_to(BASE).as_posix() if args.docker_container else str(p)
 for arch,label,target in [('x64','x64','x86_64'),('arm64','ARM64','aarch64')]:
  if args.arch not in ['both',arch]:continue
  payload=cache/('package-'+arch);m=json.loads((payload/'manifest.json').read_text());assert m['version']==VERSION and m['arch']==target
  stage=cache/('install-'+arch)
  if stage.exists():shutil.rmtree(stage)
  shutil.copytree(payload,stage,copy_function=os.link,ignore=lambda directory,names:['manifest.json'] if pathlib.Path(directory)==payload else [])
  core=cache/'target'/(target+'-pc-windows-gnullvm')/'release/diktator.exe'
  shutil.copy2(core,stage/'Silbenlot.exe')
  files={p.relative_to(stage).as_posix():{'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(stage.rglob('*')) if p.is_file()}
  marker=stage/'.diktator-installed.json';marker.write_text(json.dumps({'version':VERSION,'arch':target,'files':files},indent=2))
  # Root manifest exclusion must never exclude the runtime's nested manifest.json files.
  assert all(n in files for n in m['files'])
  meta=cache/('installer-meta-'+arch);meta.mkdir(exist_ok=True)
  all_files=sorted(files)+[marker.name];lines=[];directories=set();last=None
  for name in sorted(all_files):
   path=pathlib.PurePosixPath(name);parent=str(path.parent).replace('/','\\')
   if parent!=last:lines.append('SetOutPath "$INSTDIR'+('\\'+quote(parent) if parent!='.' else '')+'"');last=parent
   lines.append('File "${STAGE}/'+quote(name)+'"')
   for d in path.parents:
    if str(d)!='.':directories.add(str(d).replace('/','\\'))
  (meta/'payload.nsh').write_text('\n'.join(lines)+'\n',encoding='utf-8-sig')
  lines=['Delete "$INSTDIR\\'+quote(name.replace('/','\\'))+'"' for name in all_files]
  lines += ['RMDir "$INSTDIR\\'+quote(d)+'"' for d in sorted(directories,key=lambda d:(d.count('\\'),d),reverse=True)]
  (meta/'remove.nsh').write_text('\n'.join(lines)+'\n',encoding='utf-8-sig')
  outfile=out/('Silbenlot-Setup-'+label+'.exe')
  defines={'OUTPUT':outfile,'CORE':core,'STAGE':stage,'META':meta,'ICON':ROOT/'assets/silbenlot.ico','VERSION':VERSION,'ARCH':label,'SIZE_KB':(sum(v['bytes'] for v in files.values())+1023)//1024}
  command=['makensis','-V3','-WX','-INPUTCHARSET','UTF8']+[f'-D{k}={compiler_path(v) if isinstance(v,pathlib.Path) else v}' for k,v in defines.items()]+[compiler_path(ROOT/'installer/Silbenlot.nsi')]
  if args.docker_container:command=['docker','exec',args.docker_container]+command
  subprocess.run(command,check=True)
  result={'version':VERSION,'architecture':target,'installer':outfile.name,'bytes':outfile.stat().st_size,'sha256':sha(outfile),'payloadFiles':len(files),'payloadBytes':sum(v['bytes'] for v in files.values()),'defaultInstallPath':r'%LOCALAPPDATA%\Programs\Silbenlot','scope':'current user','compiler':'NSIS','payloadManifestSha256':sha(marker)}
  (out/('Silbenlot-Setup-'+label+'.json')).write_text(json.dumps(result,indent=2));print(json.dumps(result),flush=True)
if __name__=='__main__':main()
