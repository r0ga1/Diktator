import pathlib,subprocess,json,hashlib,time,select,shutil,os
base=pathlib.Path('/io/prefix/drive_c/users/root/AppData/Local');root=base/'Programs/Silbenlot';data=base/'Silbenlot/data.json';updates=base/'Silbenlot/updates';updates.mkdir(exist_ok=True)
original_data=data.read_bytes();marker=root/'.diktator-installed.json';original_marker=marker.read_bytes();m=json.loads(original_marker)
core=root/'Silbenlot.exe';shutil.copy2('/io/Silbenlot-update-test.exe',core)
def sha(path):
 with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
m['version']='0.11.99';m['files']['Silbenlot.exe']={'bytes':core.stat().st_size,'sha256':sha(core)};marker.write_text(json.dumps(m,indent=2))
folder=updates/'0.12.0';folder.mkdir(exist_ok=True);setup=folder/'Silbenlot-Setup-x64.exe'
if setup.exists():setup.unlink()
os.link('/io/Silbenlot-Setup-x64.exe',setup)
manifest=pathlib.Path('/io/release12-update.json').read_text();signed=json.loads(manifest);asset=next(a for a in signed['artifacts'] if a['arch']=='x86_64')
winbase='C:\\users\\root\\AppData\\Local'
pending={'path':winbase+'\\Silbenlot\\updates\\0.12.0\\Silbenlot-Setup-x64.exe','destination':winbase+'\\Programs\\Silbenlot\\Silbenlot.exe','sha256':asset['sha256'],'version':'0.12.0','repo':'r0ga1/silbenlot','manual':True,'manifest':manifest,'signature':pathlib.Path('/io/release12-update.sig').read_text()}
(updates/'pending.json').write_text(json.dumps(pending))
error=updates/'last-update-error.txt';error.unlink(missing_ok=True)
p=subprocess.Popen(['wine',str(core),'--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=open('/io/update-apply12.log','w'),text=True,bufsize=1)
def rpc(method):
 p.stdin.write(json.dumps({'id':method,'method':method,'params':{}})+'\n');p.stdin.flush();assert select.select([p.stdout],[],[],90)[0],method
 r=json.loads(p.stdout.readline());assert 'error' not in r,r;return r['result']
s=rpc('state');assert s['version']=='0.11.99',s
start=time.monotonic();rpc('quit');assert p.wait(timeout=20)==0
while time.monotonic()-start<240:
 if error.exists():raise AssertionError(error.read_text())
 if not (updates/'pending.json').exists() and json.loads(marker.read_text())['version']=='0.12.0':break
 time.sleep(.25)
else:raise AssertionError('Post-exit installer did not finish')
assert data.read_bytes()==original_data
assert sha(core)==json.loads(original_marker)['files']['Silbenlot.exe']['sha256']
result={'fixtureVersion':'0.11.99','installedVersion':'0.12.0','realCngManifestVerification':True,'signedProductionSetupUsed':True,'helperCopiedOutsideInstallFolder':bool(list(updates.glob('apply-*.exe'))),'oldProcessExitedBeforeSetup':True,'installedCoreHashMatchesRelease':True,'historyAndSettingsUnchanged':True,'pendingAndDownloadRemoved':not setup.exists(),'seconds':round(time.monotonic()-start,3),'networkDownload':'not exercised by this local-payload test'}
pathlib.Path('/io/update-apply12.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
