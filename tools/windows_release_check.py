#!/usr/bin/env python3
"""Destructive installation check ONLY on a fresh, disposable GitHub Windows runner.

Uses the real published 0.12 core and its native WinHTTP updater, not a version
fixture or bypass. Never run against a user's Windows profile.
"""
import argparse,ctypes,hashlib,json,os,queue,re,subprocess,sys,threading,time,urllib.request
from pathlib import Path

def wait(test, seconds, description):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        value=test()
        if value:return value
        time.sleep(.4)
    raise TimeoutError(description)

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--version',required=True);args=ap.parse_args()
    if sys.platform!='win32' or os.environ.get('GITHUB_ACTIONS')!='true':raise RuntimeError('Fresh GitHub Windows runner required; will not touch a user installation')
    assert re.fullmatch(r'\d+\.\d+\.\d+',args.version)
    result={'version':args.version,'platform':sys.platform,'checks':{}}
    out=Path('windows-check');out.mkdir(exist_ok=True)
    base=Path(os.environ['LOCALAPPDATA']);old=base/'Programs/Silbenlot';new=base/'Programs/Diktator'
    assert not (base/'Silbenlot/data.json').exists() and not (base/'Diktator/data.json').exists(),'Profile already contains application data'
    try:
        setup=Path(os.environ['RUNNER_TEMP'])/'Silbenlot-Setup-0.12.exe'
        request=urllib.request.Request('https://github.com/r0ga1/silbenlot/releases/download/v0.12.0/Silbenlot-Setup-x64.exe',headers={'User-Agent':'Diktator-Windows-check'})
        with urllib.request.urlopen(request,timeout=90) as src,setup.open('wb') as target:
            import shutil;shutil.copyfileobj(src,target)
        with setup.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()=='d1e64c3d08e8b15fef5a917976dde9a6adee935123e2767970f3f4e742781f55'
        subprocess.run([str(setup),'/S','/D='+str(old)],check=True,timeout=240)
        result['checks']['original012Installer']=True
        data=base/'Silbenlot/data.json';data.parent.mkdir(exist_ok=True)
        seed={'settings':{'threads':2,'outputMode':'clipboard','autoUpdate':True,'traySetup':True},'history':[{'id':'migration-sentinel','text':'Migrationsprüfung','created':'2026-10-25T12:00:00+02:00'}],'dictionary':[{'from':'Testwort','to':'Testwort'}]}
        data.write_text(json.dumps(seed),encoding='utf-8');original=data.read_bytes()
        class Core:
            def __init__(self,exe,dev=False):
                env=os.environ.copy()
                if dev:env['DIKTATOR_DEV_ROOT']=str(new)
                self.log=(out/('new-core.log' if dev else 'old-core.log')).open('w',encoding='utf-8')
                self.p=subprocess.Popen([str(exe),'--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.log,text=True,encoding='utf-8',env=env)
                self.q=queue.Queue();self.n=0
                def read():
                    for line in self.p.stdout:
                        try:self.q.put(json.loads(line))
                        except ValueError:pass
                threading.Thread(target=read,daemon=True).start()
            def call(self,method,params=None):
                self.n+=1;self.p.stdin.write(json.dumps({'id':self.n,'method':method,'params':params})+'\n');self.p.stdin.flush()
                reply=self.q.get(timeout=180);assert reply['id']==self.n,reply
                if 'error' in reply:raise RuntimeError(reply['error'])
                return reply['result']
            def quit(self):
                self.call('quit');self.p.wait(timeout=40);assert self.p.returncode==0;self.log.close()
        core=Core(old/'Silbenlot.exe')
        assert core.call('state')['version']=='0.12.0'
        previous_status=[None]
        def downloaded():
            s=core.call('state')
            if s['update'] != previous_status[0]:
                print(s['update'],flush=True);previous_status[0]=s['update']
            return 'geprüft' in s['update'] and args.version in s['update']
        wait(downloaded,900,'Native WinHTTP auto-update download/signature check did not finish')
        result['checks']['nativeWinHttpAutomaticDownloadAndCngSignature']=True
        core.quit()
        wait(lambda:(new/'.diktator-installed.json').exists() and json.loads((new/'.diktator-installed.json').read_text(encoding='utf-8'))['version']==args.version,300,'Renamed installation missing')
        wait(lambda:not (base/'Silbenlot/updates/pending.json').exists(),30,'Legacy update helper did not confirm installation')
        assert json.loads((old/'.diktator-installed.json').read_text(encoding='utf-8'))['version']==args.version
        assert not (old/'Silbenlot.exe').exists()
        assert data.read_bytes()==original
        migrated=json.loads((base/'Diktator/data.json').read_text(encoding='utf-8'))
        assert migrated['history'][0]['text']=='Migrationsprüfung',repr(migrated['history'][0]['text'])
        assert migrated['dictionary']==seed['dictionary'],repr(migrated['dictionary'])
        result['checks']['signed012To013FolderAndDataMigration']=True
        subprocess.run([str(new/'Diktator.exe'),'--verify-install'],check=True,timeout=90)
        # Start the ordinary installed app, without development paths or --stdio.
        gui=subprocess.Popen([str(new/'Diktator.exe')])
        ui_log=base/'Diktator/ui-start.log'
        wait(lambda:ui_log.exists() and 'READY: dashboard rendered, core connected' in ui_log.read_text(encoding='utf-8',errors='replace'),75,'Native WebView2 dashboard did not render')
        result['checks']['nativeInstalledDashboardRendered']=True
        subprocess.run([str(new/'Diktator.exe'),'--quit-existing'],check=True,timeout=45);gui.wait(timeout=30)
        core=Core(new/'Diktator.exe',True)
        s=core.call('state');assert s['version']==args.version and s['updateRepository']=='r0ga1/Diktator'
        # New process boundary must recognize actual public German audio.
        core.call('importPath',str(Path('tests/fixtures/de.wav').resolve()))
        def transcribed():
            s=core.call('state',{'includeText':True})
            if s['phase']=='Idle' and s['error']:raise AssertionError(s['error'])
            return s if s['phase']=='Idle' and 'temperatur' in s['text'].lower() else None
        transcript=wait(transcribed,180,'Supervised Parakeet transcription failed')
        result['checks']['nativeSupervisedParakeetGermanAudio']=True
        result['publicTestTranscript']=transcript['text']
        # Exercise the physical Windows hook capture, including right Ctrl.
        user=ctypes.windll.user32
        user.GetForegroundWindow.restype=ctypes.c_void_p
        user.CreateWindowExW.restype=ctypes.c_void_p
        user.CreateWindowExW.argtypes=[ctypes.c_uint32,ctypes.c_wchar_p,ctypes.c_wchar_p,ctypes.c_uint32,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_void_p]
        window=user.CreateWindowExW(0,'STATIC','Diktator Tastentest',0x10cf0000,100,100,400,150,None,None,None,None)
        assert window
        # Windows can deny SetForegroundWindow to a process which did not
        # receive the last input. Join the foreground input queue temporarily;
        # never disable the production hook's foreground-window check.
        from ctypes import wintypes
        user.GetWindowThreadProcessId.argtypes=[ctypes.c_void_p,ctypes.POINTER(wintypes.DWORD)]
        user.GetWindowThreadProcessId.restype=wintypes.DWORD
        user.SetForegroundWindow.argtypes=[ctypes.c_void_p]
        user.SetActiveWindow.argtypes=[ctypes.c_void_p]
        user.SetFocus.argtypes=[ctypes.c_void_p]
        user.ShowWindow.argtypes=[ctypes.c_void_p,ctypes.c_int]
        user.BringWindowToTop.argtypes=[ctypes.c_void_p]
        user.PeekMessageW.argtypes=[ctypes.POINTER(wintypes.MSG),ctypes.c_void_p,ctypes.c_uint,ctypes.c_uint,ctypes.c_uint]
        user.TranslateMessage.argtypes=[ctypes.POINTER(wintypes.MSG)]
        user.DispatchMessageW.argtypes=[ctypes.POINTER(wintypes.MSG)]
        user.DispatchMessageW.restype=ctypes.c_ssize_t
        focus_attempts=[]
        def focus_test_window():
            message=wintypes.MSG()
            while user.PeekMessageW(ctypes.byref(message),None,0,0,1):
                user.TranslateMessage(ctypes.byref(message));user.DispatchMessageW(ctypes.byref(message))
            foreground=user.GetForegroundWindow()
            if foreground==window:return True
            current=ctypes.windll.kernel32.GetCurrentThreadId()
            previous=user.GetWindowThreadProcessId(foreground,None) if foreground else 0
            attached=bool(previous and previous!=current and user.AttachThreadInput(current,previous,True))
            try:
                user.ShowWindow(window,5);user.BringWindowToTop(window)
                accepted=bool(user.SetForegroundWindow(window))
                user.SetActiveWindow(window);user.SetFocus(window)
                focus_attempts.append({'foreground':foreground,'thread':previous,'attached':attached,'accepted':accepted})
            finally:
                if attached:user.AttachThreadInput(current,previous,False)
            return user.GetForegroundWindow()==window
        try:wait(focus_test_window,10,'Cannot focus keyboard test window')
        finally:result['keyboardFocusAttempts']=focus_attempts[-3:]
        core.call('attachWindow',window)
        class KBD(ctypes.Structure):_fields_=[('vk',ctypes.c_ushort),('scan',ctypes.c_ushort),('flags',ctypes.c_uint32),('time',ctypes.c_uint32),('extra',ctypes.c_size_t)]
        class UNION(ctypes.Union):_fields_=[('ki',KBD),('padding',ctypes.c_byte*32)]
        class INPUT(ctypes.Structure):_fields_=[('type',ctypes.c_uint32),('u',UNION)]
        def key(vk,up=False,extended=False):
            v=INPUT();v.type=1;v.u.ki=KBD(vk,0,(2 if up else 0)|(1 if extended else 0),0,0)
            assert user.SendInput(1,ctypes.byref(v),ctypes.sizeof(v))==1
            time.sleep(.08)
        core.call('shortcutCapture','record');key(163,extended=True);key(163,True,True)
        wait(lambda:core.call('state')['settings']['shortcut']['key']==163,5,'Right Ctrl was not preserved')
        core.call('shortcutCapture','record');key(163,extended=True);key(120);key(120,True);key(163,True,True)
        wait(lambda:core.call('state')['settings']['shortcut']=={'key':120,'modifiers':32},5,'Right Ctrl combination was not preserved')
        result['checks']['nativeRightCtrlAndRightCtrlF9Capture']=True
        core.call('attachWindow',0);user.DestroyWindow(ctypes.c_void_p(window))
        assert core.call('history',{})['total']>=2
        core.call('clearHistory',{'confirm':True})
        assert core.call('history',{})['total']==0 and core.call('dashboard')['sessions']==0
        assert core.call('dictionary')==seed['dictionary']
        result['checks']['clearHistoryPreservesDictionary']=True
        core.call('releaseModel')
        wait(lambda:not core.call('state')['modelLoaded'],15,'Inference worker did not unload')
        from ctypes import wintypes
        class MEMORY(ctypes.Structure):
            _fields_=[('cb',wintypes.DWORD),('faults',wintypes.DWORD),('peak',ctypes.c_size_t),('working',ctypes.c_size_t),('pool1',ctypes.c_size_t),('pool2',ctypes.c_size_t),('pool3',ctypes.c_size_t),('pool4',ctypes.c_size_t),('pagefile',ctypes.c_size_t),('peakfile',ctypes.c_size_t)]
        handle=ctypes.c_void_p(int(core.p._handle))
        def cpu_time():
            times=[wintypes.FILETIME() for _ in range(4)]
            assert ctypes.windll.kernel32.GetProcessTimes(handle,*[ctypes.byref(x) for x in times])
            return sum((x.dwHighDateTime<<32)|x.dwLowDateTime for x in times[2:])/1e7
        start_cpu=cpu_time();start_wall=time.monotonic();time.sleep(2);cpu_percent=(cpu_time()-start_cpu)/(time.monotonic()-start_wall)*100
        mem=MEMORY();mem.cb=ctypes.sizeof(mem)
        assert ctypes.windll.psapi.GetProcessMemoryInfo(handle,ctypes.byref(mem),mem.cb)
        result['idle']={'modelUnloaded':True,'coreWorkingSetMiB':round(mem.working/1048576,2),'coreCpuPercentOfOneLogicalCore':round(cpu_percent,3),'measurementSeconds':2,'scope':'core with dashboard closed; Windows CI VM, not a hardware minimum'}
        core.quit()
        result['success']=True
    except Exception as e:
        result['success']=False;result['error']=repr(e)
        raise
    finally:
        (out/'result.json').write_text(json.dumps(result,indent=2,ensure_ascii=False),encoding='utf-8')
        for name in ['Diktator','Silbenlot']:
            p=base/name/'ui-start.log'
            if p.exists():(out/(name+'-ui.log')).write_bytes(p.read_bytes())
        print(json.dumps(result,ensure_ascii=False),flush=True)
if __name__=='__main__':main()
