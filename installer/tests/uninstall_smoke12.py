import pathlib,subprocess,hashlib,json,shutil,time
base=pathlib.Path('/io/prefix/drive_c/users/root');root=base/'AppData/Local/Programs/Silbenlot';data=base/'AppData/Local/Silbenlot/data.json';menu=base/'AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Silbenlot'
old=hashlib.sha256(data.read_bytes()).hexdigest();shutil.copy2(root/'Uninstall.exe','/io/Silbenlot-Uninstall-Test.exe')
# Exercise uninstallation while the real installed tray worker is still running.
app=subprocess.Popen(['wine',str(root/'Silbenlot.exe'),'--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,bufsize=1)
app.stdin.write(json.dumps({'id':'state','method':'state','params':{}})+'\n');app.stdin.flush()
reply=json.loads(app.stdout.readline());assert reply['result']['version']=='0.12.0' and reply['result']['phase']=='Idle'
start=time.monotonic()
p=subprocess.run(['wine','/io/Silbenlot-Uninstall-Test.exe','/S',r'_?=C:\users\root\AppData\Local\Programs\Silbenlot'],stdout=open('/io/uninstall12.log','w'),stderr=subprocess.STDOUT,timeout=90)
assert p.returncode==0,p.returncode
assert not root.exists(),list(root.rglob('*')) if root.exists() else ''
assert not menu.exists()
assert data.exists() and hashlib.sha256(data.read_bytes()).hexdigest()==old
for key,value in [(r'HKCU\Software\Silbenlot',None),(r'HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot',None),(r'HKCU\Software\Microsoft\Windows\CurrentVersion\Run','Silbenlot')]:
 args=['wine','reg','query',key]+(['/v',value] if value else [])
 assert subprocess.run(args,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode!=0,args
assert app.wait(timeout=15)==0
result={'runningCoreExitedGracefully':True,'uninstallExitCode':p.returncode,'seconds':round(time.monotonic()-start,3),'programDirectoryRemoved':True,'startMenuRemoved':True,'registryAndOwnedAutostartRemoved':True,'personalDataPreserved':True,'personalDataSha256':old}
pathlib.Path('/io/uninstall12.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
