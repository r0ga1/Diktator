import subprocess,json,time,select,pathlib,hashlib
root=pathlib.Path('/io/prefix/drive_c/users/root/AppData/Local/Programs/Silbenlot');data=root.parents[1]/'Silbenlot/data.json'
old=hashlib.sha256(data.read_bytes()).hexdigest()
p=subprocess.Popen(['wine',str(root/'Silbenlot.exe'),'--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=open('/io/installed-model12.log','w'),text=True,bufsize=1)
def send(m,v=None):p.stdin.write(json.dumps({'id':m,'method':m,'params':v or {}})+'\n');p.stdin.flush()
def reply():
 assert select.select([p.stdout],[],[],180)[0],'No core reply'
 r=json.loads(p.stdout.readline());assert 'error' not in r,r;return r['result']
def rpc(m,v=None):send(m,v);return reply()
s=rpc('state');assert s['version']=='0.12.0' and not s['modelLoaded']
before=rpc('history')['total'];send('import');print('IMPORT_DIALOG_PENDING',flush=True);assert reply()==True
start=time.monotonic()
while time.monotonic()-start<120:
 s=rpc('state')
 assert not s['error'],s
 if s['phase']=='Idle' and rpc('history')['total']==before+1:break
 time.sleep(.3)
else:raise RuntimeError('Model/import timeout')
h=rpc('history');e=h['entries'][0];assert e['text']=='Alles hat ein Ende, nur die Wurst hat zwei.',e
result={'version':'0.12.0','root':str(root),'developmentRootOverride':False,'model': 'installed Parakeet v3 mixed precision','referenceLanguage':'German','text':e['text'],'modelLoaded':s['modelLoaded'],'seconds':round(time.monotonic()-start,3)}
rpc('deleteEntry',{'id':e['id']});rpc('quit');assert p.wait(timeout=20)==0
result['originalHistoryAndSettingsRestored']=hashlib.sha256(data.read_bytes()).hexdigest()==old
assert result['originalHistoryAndSettingsRestored']
pathlib.Path('/io/installed-model12.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2),flush=True)
