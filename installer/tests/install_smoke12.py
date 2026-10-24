import subprocess,json,time,pathlib,hashlib,select,os
base=pathlib.Path('/io/prefix/drive_c/users/root')
root=base/'AppData/Local/Programs/Silbenlot';data=base/'AppData/Local/Silbenlot/data.json'
cache=base/'AppData/Local/Silbenlot/cache';before_cache={p.name for p in cache.iterdir()} if cache.exists() else set()
setup='/io/Silbenlot-Setup-x64.exe';exe=str(root/'Silbenlot.exe');winexe=r'C:\users\root\AppData\Local\Programs\Silbenlot\Silbenlot.exe'
log=open('/io/installer12-smoke.log','w');result={}
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def run(args,timeout=240):
 p=subprocess.run(['wine']+args,stdout=log,stderr=subprocess.STDOUT,timeout=timeout);return p.returncode
def reg(*args):
 p=subprocess.run(['wine','reg']+list(args),stdout=subprocess.PIPE,stderr=log,text=True,timeout=40);return p.returncode,p.stdout
start=time.monotonic();old_data=sha(data) if data.exists() else None
assert run([setup,'/S'])==0,'Install failed: see installer12-smoke.log'
result['installSeconds']=round(time.monotonic()-start,3)
assert (root/'Uninstall.exe').is_file() and (root/'.verified-cache.json').is_file()
manifest=json.loads((root/'.diktator-installed.json').read_text());assert manifest['version']=='0.12.0'
for name,v in manifest['files'].items():
 p=root/name;assert p.stat().st_size==v['bytes'] and sha(p)==v['sha256'],name
result['verifiedInstalledFiles']=len(manifest['files'])
assert old_data is None or sha(data)==old_data,'Install changed personal data'
code,info=reg('query',r'HKCU\Software\Silbenlot','/v','InstallDir');assert code==0 and winexe.rsplit('\\',1)[0].lower() in info.lower(),info
code,info=reg('query',r'HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot','/v','DisplayVersion');assert code==0 and '0.12.0' in info,info
menu=base/'AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Silbenlot'
assert (menu/'Silbenlot.lnk').is_file() and (menu/'Silbenlot deinstallieren.lnk').is_file()
result['startMenuAndUninstallerRegistered']=True

def launch():
 p=subprocess.Popen(['wine',exe,'--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=log,text=True,bufsize=1)
 return p
def rpc(p,m):
 p.stdin.write(json.dumps({'id':m,'method':m,'params':{}})+'\n');p.stdin.flush()
 assert select.select([p.stdout],[],[],90)[0],'No reply to '+m
 v=json.loads(p.stdout.readline());assert 'error' not in v,v;return v['result']
p=launch();start=time.monotonic();s=rpc(p,'state');result['secondsToCoreState']=round(time.monotonic()-start,3)
assert s['version']=='0.12.0' and s['ready'] and not s['modelLoaded'],s
result['state']={k:s[k] for k in ['version','arch','phase','ready','modelLoaded','shortcut']}
result['historyEntries']=rpc(p,'history')['total']
# An ordinary file change must fail verification and be repaired by the setup.
html=root/'ui/index.html';expected=sha(html);html.write_bytes(html.read_bytes()+b'\ncorrupted\n')
assert run([exe,'--verify-install'])==10,'Changed UI must fail installation verification'
reg('add',r'HKCU\Software\Microsoft\Windows\CurrentVersion\Run','/v','Silbenlot','/t','REG_SZ','/d',r'"Z:\io\OldPortable.exe" --autostart','/f')
assert run([setup,'/S'])==0,'Repair failed'
assert p.wait(timeout=15)==0,'Installer did not finish old process gracefully'
assert sha(html)==expected
assert old_data is None or sha(data)==old_data,'Repair changed personal data'
code,info=reg('query',r'HKCU\Software\Microsoft\Windows\CurrentVersion\Run','/v','Silbenlot');assert code==0 and winexe.lower() in info.lower(),info
result['repairRestoresChangedFile']=True;result['runningCoreExitedGracefully']=True;result['existingAutostartMovedToInstalledPath']=True
p=launch();s=rpc(p,'state');assert s['version']=='0.12.0' and not s['modelLoaded'];rpc(p,'quit');assert p.wait(timeout=15)==0
assert before_cache==({p.name for p in cache.iterdir()} if cache.exists() else set()),'Installed app created a portable model cache'
result['noAdditionalPortableCache']=True
result['personalDataSha256']=sha(data) if data.exists() else None
result['nativeDashboard']='not-tested-by-this-core-script'
pathlib.Path('/io/installer12-smoke.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2),flush=True)
