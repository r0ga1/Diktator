#!/usr/bin/env python3
"""Validate and locally sign Diktator release assets, optionally upload a beta.

The private key never goes to GitHub. A source-code push alone is not an update.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import zipfile

ROOT=Path(__file__).resolve().parents[1]
def sha(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def run(command,**kwargs):return subprocess.run(command,check=True,**kwargs)
def sign(body, out, key):
    (out/'update.json').write_bytes(body)
    with tempfile.TemporaryDirectory() as temporary:
        sig=Path(temporary)/'signature';pub=Path(temporary)/'public.pem'
        run(['openssl','dgst','-sha256','-sign',str(key),'-sigopt','rsa_padding_mode:pss','-sigopt','rsa_pss_saltlen:32','-out',str(sig),str(out/'update.json')])
        run(['openssl','pkey','-in',str(key),'-pubout','-out',str(pub)],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
        run(['openssl','dgst','-sha256','-verify',str(pub),'-sigopt','rsa_padding_mode:pss','-sigopt','rsa_pss_saltlen:32','-signature',str(sig),str(out/'update.json')])
        (out/'update.sig').write_text(sig.read_bytes().hex()+'\n')

def publish(repo,tag,out,uploaded,notes,title):
    # No updater sees a partial release: upload to draft, verify every asset, publish last.
    run(['gh','release','create',tag,'--repo',repo,'--verify-tag','--draft','--prerelease','--title',title,'--notes-file',str(notes),*[str(out/n) for n in uploaded]])
    releases=json.loads(run(['gh','api',f'repos/{repo}/releases?per_page=100'],stdout=subprocess.PIPE).stdout)
    matches=[release for release in releases if release['tag_name']==tag]
    if len(matches)!=1 or not matches[0]['draft']:raise ValueError('Expected one draft before upload validation')
    assets={a['name']:a for a in matches[0]['assets']}
    if set(assets)!=set(uploaded):raise ValueError('Remote asset set differs')
    for name in uploaded:
        a=assets[name]
        if a['state']!='uploaded' or a['size']!=(out/name).stat().st_size:raise ValueError('Incomplete upload: '+name)
        if a.get('digest') and a['digest']!='sha256:'+sha(out/name):raise ValueError('Remote digest mismatch: '+name)
    run(['gh','release','edit',tag,'--repo',repo,'--draft=false','--prerelease'])

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,required=True)
    p.add_argument('--key',type=Path,default=Path.home()/'.local/share/silbenlot-release/update-signing.pem')
    p.add_argument('--publish-beta',action='store_true')
    p.add_argument('--legacy-bridge',action='store_true',help='One-time signed 0.12 Silbenlot -> 0.13 Diktator transition')
    args=p.parse_args();out=args.assets.resolve();cfg=json.loads((ROOT/'release-config.json').read_text())
    version=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M).group(1)
    if not re.fullmatch(r'\d+\.\d+\.\d+',version):raise ValueError('Three-part release version required')
    if sha(ROOT/'licenses/WebView2-FixedVersion-Microsoft.txt')!=cfg['runtimeTermsSha256']:raise ValueError('Runtime terms digest changed; review config and installer terms')
    artifacts=[]
    for arch,label in [('x86_64','x64'),('aarch64','ARM64')]:
        file=out/f'Diktator-Setup-{label}.exe';meta=json.loads(file.with_suffix('.json').read_text())
        if meta['version']!=version or meta['architecture']!=arch or meta['sha256']!=sha(file) or meta['bytes']!=file.stat().st_size:raise ValueError('Setup metadata/hash mismatch')
        if not 0<file.stat().st_size<=1_800_000_000:raise ValueError('Setup exceeds updater limit')
        artifacts.append({'arch':arch,'file':file.name,'bytes':file.stat().st_size,'sha256':sha(file)})
    source=out/'Diktator-Source.zip'
    with zipfile.ZipFile(source) as z:
        expected={}
        for path in ROOT.rglob('*'):
            rel=path.relative_to(ROOT)
            if path.is_file() and (rel.parts[0]=='vendor' or not any(part in ('target','dist','.git','__pycache__') for part in rel.parts)):
                expected['Diktator-Source/'+rel.as_posix()]=path
        if len(z.namelist())!=len(set(z.namelist())) or set(z.namelist())!=set(expected):raise ValueError('Source archive file set is stale or duplicated')
        for name,path in expected.items():
            with z.open(name) as member:
                if hashlib.file_digest(member,'sha256').hexdigest()!=sha(path):raise ValueError('Source archive is stale: '+name)
        native=json.loads((ROOT/'third_party/native-sources.json').read_text())
        for item in native:
            data=z.read('Diktator-Source/third_party/source-archives/'+item['file'])
            if hashlib.sha256(data).hexdigest()!=item['sha256']:raise ValueError('Native source snapshot mismatch')
        if any(n.lower().endswith(('.pem','.pfx','.p12')) for n in z.namelist() if '/vendor/' not in n):raise ValueError('Private-key-like file in source archive')
    manifest={'schema':1,'repository':cfg['repository'],'channel':cfg['channel'],'keyId':cfg['keyId'],'version':version,'runtimeTermsSha256':cfg['runtimeTermsSha256'],'artifacts':artifacts,'source':{'arch':'source','file':source.name,'bytes':source.stat().st_size,'sha256':sha(source)}}
    body=json.dumps(manifest,sort_keys=True,separators=(',',':')).encode()+b'\n';(out/'update.json').write_bytes(body)
    # The key ID must match the compiled publisher key; fail instead of key rotation.
    public=run(['openssl','pkey','-in',str(args.key),'-pubout','-outform','DER'],stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout
    if hashlib.sha256(public).hexdigest()[:20]!=cfg['keyId']:raise ValueError('Wrong signing key; compiled app trusts a different publisher')
    sign(body,out,args.key)
    names=[a['file'] for a in artifacts]+[source.name,'update.json','update.sig']
    (out/'Diktator-SHA256SUMS.txt').write_text(''.join(sha(out/name)+'  '+name+'\n' for name in names))
    if args.publish_beta:
        notes=out/'Diktator-Release-Notes.md'
        if not notes.is_file():raise ValueError('Review Diktator-Release-Notes.md before publishing')
        publish(cfg['repository'],'v'+version,out,names+['Diktator-SHA256SUMS.txt'],notes,f'Diktator {version} – Beta')
    if args.legacy_bridge:
        if version!='0.13.0' or cfg['repository']!='r0ga1/Diktator':raise ValueError('Legacy bridge is limited to the reviewed 0.13 transition')
        import copy,os,shutil
        legacy=out/('legacy-'+version);legacy.mkdir(exist_ok=True)
        bridge=copy.deepcopy(manifest);bridge['repository']='r0ga1/silbenlot'
        legacy_names=[]
        for a in bridge['artifacts']+[bridge['source']]:
            current=out/a['file'];a['file']=a['file'].replace('Diktator-','Silbenlot-');dest=legacy/a['file']
            if dest.exists():dest.unlink()
            try:os.link(current,dest)
            except OSError:shutil.copyfile(current,dest)
            legacy_names.append(a['file'])
        sign(json.dumps(bridge,sort_keys=True,separators=(',',':')).encode()+b'\n',legacy,args.key)
        legacy_names+=['update.json','update.sig']
        sums='Silbenlot-SHA256SUMS.txt';(legacy/sums).write_text(''.join(sha(legacy/n)+'  '+n+'\n' for n in legacy_names))
        notes=legacy/'bridge-notes.md'
        notes.write_text('# Diktator 0.13 · Umstieg von Silbenlot\n\nDie App heißt auf Nutzerwunsch wieder **Diktator**. Weiterentwicklung, Downloads und Dokumentation: [r0ga1/Diktator](https://github.com/r0ga1/Diktator).\n\nBestehende 0.12-Installationen erhalten hier ein signiertes Übergangsupdate. Das Setup übernimmt Daten und Verknüpfungen und wechselt zum Diktator-Ordner. Der alte persönliche Datenordner bleibt erhalten. Künftige Updates kommen aus dem neuen Repository. Die Dateinamen hier bleiben ausschließlich für den bisherigen Updater erhalten.\n\nBeta; Prüfstand und Grenzen im [Diktator-Release](https://github.com/r0ga1/Diktator/releases/tag/v0.13.0).\n')
        if args.publish_beta:publish('r0ga1/silbenlot','v'+version,legacy,legacy_names+[sums],notes,'Diktator 0.13 – Übergangsupdate')
    print('Validated and signed release',version,'for',cfg['repository'],'(private key retained locally)')
if __name__=='__main__':main()
