#!/usr/bin/env python3
"""Build two Rust portable EXEs with a verified, embedded Parakeet v3 bundle."""
import argparse,hashlib,json,os,pathlib,re,shutil,struct,subprocess,tarfile,urllib.request,zipfile
ROOT=pathlib.Path(__file__).resolve().parent
VERSION=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M).group(1)
ASSETS={
 'silero_vad.onnx':('https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx','9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6'),
 'decoder.onnx':('https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3/resolve/1a468a35cbba69418f126de829e75261dea4a4e4/decoder.onnx','d593cdb0e571f5a457ec2219af9968cbf6b0e8198e8f7839b40a8754593bf68c'),
 'joiner.onnx':('https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3/resolve/1a468a35cbba69418f126de829e75261dea4a4e4/joiner.onnx','b9b0bcf88ac571902e69a6536223ed2d94885e981b85045410f1403d53121a63'),
 'model.tar.bz2':('https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2','5793d0fd397c5778d2cf2126994d58e9d56b1be7c04d13c7a15bb1b4eafb16bf'),
 'x64.tar.bz2':('https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.8/sherpa-onnx-v1.13.8-win-x64-shared-MT-MinSizeRel-no-tts.tar.bz2','a0ec3b679b6da3da5d79b398b0ed696e19c594c9f26ab03e6ce551ac4e55f9a4'),
 'arm64.tar.bz2':('https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.8/sherpa-onnx-v1.13.8-win-arm64-shared-MT-MinSizeRel-no-tts.tar.bz2','755a4de5f9bbe0ff6b797119216c9533c143291b5129949c10db6f7325623502'),
 'llvm-mingw.tar.xz':('https://github.com/mstorsjo/llvm-mingw/releases/download/20261020/llvm-mingw-20261020-ucrt-macos-universal.tar.xz','e9c13e4ede38e715003ffc674d4d00a1088df8235439d1f0c16188ff0704c17e')}
WEBVIEW=json.loads((ROOT/'webview-runtime.json').read_text())
for arch,v in WEBVIEW.items():ASSETS['webview2-'+arch+'.cab']=(v['url'],v['sha256'])
ASSETS['webview2-sdk.nupkg']=('https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/1.0.4314.35/microsoft.web.webview2.1.0.4314.35.nupkg','90778deb1b77e36292828b4b5833c9255c27533f40e0ddd55891ae718944edc4')
MAGIC=b'DIKTATOR-BUNDLE1'
def sha(path):
 h=hashlib.sha256()
 with pathlib.Path(path).open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
def obtain(name,cache,shared):
 p=cache/'downloads'/name
 if shared and (shared/'downloads'/name).is_file():p=shared/'downloads'/name
 if not p.is_file():
  p.parent.mkdir(parents=True,exist_ok=True);part=p.with_suffix('.partial');print('Downloading',name,flush=True)
  with urllib.request.urlopen(ASSETS[name][0],timeout=120) as src,part.open('wb') as dst:shutil.copyfileobj(src,dst)
  part.replace(p)
 if sha(p)!=ASSETS[name][1]:raise RuntimeError('Asset checksum mismatch: '+name)
 return p

def extract(archive,dest,names,part=None):
 found=set();dest.mkdir(parents=True,exist_ok=True)
 with tarfile.open(archive,'r:*') as t:
  for m in t:
   name=pathlib.PurePosixPath(m.name).name
   if m.isfile() and name in names and (part is None or part in m.name):
    with t.extractfile(m) as src,(dest/name).open('wb') as dst:shutil.copyfileobj(src,dst)
    found.add(name)
 if found!=set(names):raise RuntimeError('Missing files: '+str(set(names)-found))
def verify_arch(folder,machine):
 for f in folder.rglob('*'):
  if f.suffix.lower() not in ('.exe','.dll'):continue
  with f.open('rb') as src:
   head=src.read(64);src.seek(int.from_bytes(head[60:64],'little'));pe=src.read(6)
   if head[:2]!=b'MZ' or pe[:4]!=b'PE\0\0' or int.from_bytes(pe[4:6],'little')!=machine:raise RuntimeError('Architecture mismatch: '+str(f))
def resource(cache,compiler,arch):
 p=cache/('resource-'+arch);p.mkdir(parents=True,exist_ok=True);version=VERSION.replace('.',',')+',0'
 manifest='''<?xml version="1.0" encoding="UTF-8" standalone="yes"?><assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0"><assemblyIdentity version="VERSION.0" processorArchitecture="*" name="Diktator" type="win32"/><description>Diktator</description><dependency><dependentAssembly><assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/></dependentAssembly></dependency><trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo><application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings><dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2,PerMonitor</dpiAwareness><longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware></windowsSettings></application><compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1"><application><supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/></application></compatibility></assembly>'''.replace('VERSION',VERSION)
 (p/'app.manifest').write_text(manifest)
 rc='''1 ICON "ICON"
1 24 "app.manifest"
1 VERSIONINFO
FILEVERSION VERSIONNUM
PRODUCTVERSION VERSIONNUM
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo" BEGIN
  BLOCK "040904b0" BEGIN
   VALUE "CompanyName", "Diktator contributors\\0"
   VALUE "FileDescription", "Diktator - lokales Diktat\\0"
   VALUE "FileVersion", "VERSIONTEXT\\0"
   VALUE "InternalName", "Diktator\\0"
   VALUE "OriginalFilename", "Diktator.exe\\0"
   VALUE "ProductName", "Diktator\\0"
   VALUE "ProductVersion", "VERSIONTEXT\\0"
   VALUE "LegalCopyright", "GPL-3.0; third-party notices included\\0"
  END
 END
 BLOCK "VarFileInfo" BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
'''.replace('ICON"','ICON"').replace('"ICON"','"'+str(ROOT/'assets/diktator.ico')+'"').replace('VERSIONNUM',version).replace('VERSIONTEXT',VERSION)
 (p/'app.rc').write_text(rc);key=hashlib.sha256((rc+manifest).encode()+(ROOT/'assets/diktator.ico').read_bytes()).hexdigest()[:16];obj=p/('app-'+key+'.o');subprocess.run([str(compiler),'-i',str(p/'app.rc'),'-o',str(obj),'-O','coff'],cwd=p,check=True);return obj

def source_zip(out):
 p=out/'Diktator-Source.zip'
 with zipfile.ZipFile(p,'w',zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as z:
  for f in sorted(ROOT.rglob('*')):
   rel=f.relative_to(ROOT)
   # Vendored packages can legitimately contain source folders named target/.
   # Keep every crate file so Cargo's package checksums also work after unzip.
   if f.is_file() and (rel.parts[0]=='vendor' or not any(x in ('target','dist','.git','__pycache__') for x in rel.parts)):z.write(f,'Diktator-Source/'+rel.as_posix())
 return p

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--cache',type=pathlib.Path,required=True);ap.add_argument('--out',type=pathlib.Path,required=True);ap.add_argument('--shared-cache',type=pathlib.Path);ap.add_argument('--toolchain',type=pathlib.Path);ap.add_argument('--layout-only',action='store_true',help='Prepare verified payload directories without portable compression');ap.add_argument('--arch',choices=['both','x64','arm64'],default='both');args=ap.parse_args();cache=args.cache.resolve();cache.mkdir(parents=True,exist_ok=True);out=args.out.resolve();out.mkdir(parents=True,exist_ok=True);shared=args.shared_cache.resolve() if args.shared_cache else None
 if args.toolchain:toolchain=args.toolchain.resolve()
 else:
  path=obtain('llvm-mingw.tar.xz',cache,shared);dest=cache/'toolchain';dest.mkdir(exist_ok=True)
  if not (dest/'llvm-mingw-20261020-ucrt-macos-universal/bin').is_dir():
   with tarfile.open(path,'r:xz') as t:t.extractall(dest,filter='data')
  toolchain=dest/'llvm-mingw-20261020-ucrt-macos-universal'
 cargo=shutil.which('cargo')
 if not cargo:raise RuntimeError('Rust/Cargo required')
 models=cache/'model'
 if models.exists():shutil.rmtree(models)
 extract(obtain('model.tar.bz2',cache,shared),models,['encoder.int8.onnx','tokens.txt'])
 for name in ['decoder.onnx','joiner.onnx']:shutil.copy2(obtain(name,cache,shared),models/name)
 shutil.copy2(obtain('silero_vad.onnx',cache,shared),models/'silero_vad.onnx')
 for arch,label,target,machine in [('x64','x64','x86_64',0x8664),('arm64','ARM64','aarch64',0xaa64)]:
  if args.arch!='both' and args.arch!=arch:continue
  print('Building Rust',label,flush=True);triple=target+'-pc-windows-gnullvm';prefix=target+'-w64-mingw32';env=os.environ.copy();env['PATH']=str(toolchain/'bin')+os.pathsep+env.get('PATH','');env['CARGO_TARGET_DIR']=str(cache/'target');env['CARGO_TARGET_'+triple.upper().replace('-','_')+'_LINKER']=str(toolchain/'bin'/(prefix+'-clang'))
  obj=resource(cache,toolchain/'bin'/(prefix+'-windres'),arch);env['CARGO_ENCODED_RUSTFLAGS']='\x1f'.join(['-C','target-feature=+crt-static','-C','link-arg='+str(obj)])
  subprocess.run([cargo,'build','--locked','--release','--target',triple],cwd=ROOT,env=env,check=True)
  app=cache/'target'/triple/'release/diktator.exe';stage=cache/('package-'+arch)
  go=shutil.which('go')
  if not go:raise RuntimeError('Go 1.26+ required for the native WebView2 shell')
  goarch='amd64' if arch=='x64' else 'arm64'
  shutil.copy2(obj,ROOT/'shell'/('rsrc_windows_'+goarch+'.syso'))
  goenv=os.environ.copy();goenv.update(GOOS='windows',GOARCH=goarch,CGO_ENABLED='0',GOCACHE=str(cache/'go-cache'),GOPATH=str(cache/'go-path'))
  shell=cache/('ui-'+arch+'.exe')
  subprocess.run([go,'build','-mod=vendor','-trimpath','-ldflags=-s -w -H=windowsgui -buildid=','-o',str(shell),'.'],cwd=ROOT/'shell',env=goenv,check=True)
  if stage.exists():shutil.rmtree(stage)
  shutil.copytree(models,stage/'model')
  (stage/'ui').mkdir();shutil.copy2(shell,stage/'ui/Diktator-UI.exe')
  with zipfile.ZipFile(obtain('webview2-sdk.nupkg',cache,None)) as sdk:
   (stage/'ui/WebView2Loader.dll').write_bytes(sdk.read('build/native/'+arch+'/WebView2Loader.dll'))
  import base64
  html=(ROOT/'shell/ui/index.html').read_text().replace('/* INLINE_CSS */',(ROOT/'shell/ui/style.css').read_text()).replace('FONT_DATA',base64.b64encode((ROOT/'shell/ui/rounded.ttf').read_bytes()).decode()).replace('LOGO_DATA',base64.b64encode((ROOT/'shell/ui/logo.png').read_bytes()).decode()).replace('/* INLINE_JS */',(ROOT/'shell/ui/app.js').read_text()).replace('/* BRIDGE */',(ROOT/'shell/ui/native-bridge.js').read_text())
  (stage/'ui/index.html').write_text(html)
  extract(obtain(arch+'.tar.bz2',cache,shared),stage/'runtime',['sherpa-onnx-c-api.dll','onnxruntime.dll','onnxruntime_providers_shared.dll'],'/lib/')
  shutil.copytree(ROOT/'licenses',stage/'licenses');shutil.copy2(ROOT/'LICENSE',stage/'LICENSE');shutil.copy2(ROOT/'README.md',stage/'README.md');shutil.copy2(ROOT/'THIRD_PARTY.md',stage/'THIRD_PARTY.md')
  for name in ['ADDITIONAL_PERMISSION.md','PRIVACY.md','PROVENANCE.md','NOTICE','RELEASE-REVIEW.md']:shutil.copy2(ROOT/name,stage/name)
  cab=obtain('webview2-'+arch+'.cab',cache,None)
  expanded=cache/('webview-'+arch)
  runtime=expanded/('Microsoft.WebView2.FixedVersionRuntime.'+WEBVIEW[arch]['version']+'.'+arch)
  if not runtime.is_dir():
   expanded.mkdir(parents=True,exist_ok=True)
   subprocess.run(['bsdtar','-xf',str(cab),'-C',str(expanded)],check=True)
  shutil.copytree(runtime,stage/'webview2')
  # The browser runtime can include helper binaries of other architectures.
  # Check our own binaries and its main executable against the selected official CAB.
  verify_arch(stage/'runtime',machine);verify_arch(stage/'ui',machine);verify_arch(app.parent,machine)
  verify_arch_file=stage/'webview2/msedgewebview2.exe'
  with verify_arch_file.open('rb') as f:
   head=f.read(64);f.seek(int.from_bytes(head[60:64],'little'));pe=f.read(6)
   if pe[:4]!=b'PE\0\0' or int.from_bytes(pe[4:6],'little')!=machine:raise RuntimeError('WebView2 architecture mismatch')
  sums={p.relative_to(stage).as_posix():{'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(stage.rglob('*')) if p.is_file()};manifest={'version':VERSION,'arch':target,'files':sums};(stage/'manifest.json').write_text(json.dumps(manifest,indent=2))
  if args.layout_only:
   print('Prepared',stage,flush=True)
   continue
  payload=cache/(arch+'-payload.zip')
  with zipfile.ZipFile(payload,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
   for p in sorted(stage.rglob('*')):
    if p.is_file():z.write(p,p.relative_to(stage).as_posix())
  exe=out/('Diktator-'+label+'.exe')
  with exe.open('wb') as dst:
   for f in [app,payload]:
    with f.open('rb') as src:shutil.copyfileobj(src,dst)
   dst.write(MAGIC);dst.write(struct.pack('<Q',payload.stat().st_size));dst.write(bytes.fromhex(sha(payload)))
  print(exe.name,f'{exe.stat().st_size/1e6:.1f} MB',flush=True)
 source_zip(out)
 (out/'SHA256SUMS.txt').write_text(''.join(sha(p)+'  '+p.name+'\n' for p in sorted(out.iterdir()) if p.is_file() and (p.name.startswith('Diktator-') and p.suffix in ('.exe','.zip'))))
if __name__=='__main__':main()
