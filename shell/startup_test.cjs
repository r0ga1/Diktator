// Regression for the real hidden-window bootstrap deadlock. No production Node dependency.
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const mode=process.env.DIKTATOR_TEST_STARTUP||'ok';
const nodes=new Map(),listeners={},timers=new Map();let tick=0,ready=0,states=0;
const node=q=>{if(!nodes.has(q))nodes.set(q,{innerHTML:'',textContent:'',className:'',scrollTop:0,hidden:false,children:[{}],classList:{toggle(){}},dataset:{},style:{}});return nodes.get(q)};
const document={hidden:true,fonts:{ready:Promise.resolve()},querySelector:node,querySelectorAll:()=>[],addEventListener:(n,f)=>{(listeners[n]??=[]).push(f)}};
const state={version:'0.13.0',phase:'Idle',ready:true,modelLoaded:false,settings:{startVolume:35,stopVolume:35},shortcut:'F9',cancelShortcut:'Esc',historyStamp:'0',text:'',status:'Bereit'};
const window={addEventListener(){},nativeReady:async()=>{ready++},nativeCall:async method=>{if(method==='state'){states++;if(mode==='core-error')throw Error('core offline');return state}if(method==='dashboard'){if(mode==='render-error')throw Error('dashboard offline');return{saved:0,words:0,sessions:0,wpm:0,keystrokes:0,recent:[]}};throw Error(method)}};
const context={document,window,console,setTimeout:(f,ms)=>{timers.set(++tick,{f,ms});return tick},clearTimeout:id=>timers.delete(id),Date,Number,String,Math,JSON};
(async()=>{vm.runInNewContext(fs.readFileSync(process.argv[2]||__dirname+'/ui/app.js','utf8'),context);await new Promise(setImmediate);await new Promise(setImmediate);
 if(mode!=='ok'){assert.equal(ready,0,'failed core/render must never signal a working dashboard');console.log('PASS: '+mode+' cannot report nativeReady');return;}
 assert.equal(ready,1,'hidden window must reach nativeReady without waiting for visibility');assert.equal(states,1);assert.match(node('#content').innerHTML,/Dein erstes Diktat/);assert.equal(timers.size,0,'no permanent polling in hidden idle window');
 document.hidden=false;listeners.visibilitychange[0]();await new Promise(setImmediate);assert.equal(states,2);assert.equal(timers.size,1);
 document.hidden=true;listeners.visibilitychange[0]();assert.equal(timers.size,0,'hide stops idle polling');
 vm.runInNewContext("page='settings';settingsTab='updates';renderSettings()",context);
 assert.match(node('#settingsSection').innerHTML,/data-setting="autoUpdate"/);
 assert.doesNotMatch(node('#settingsSection').innerHTML,/githubRepo|saveRepo|type=\"text\"/);
 vm.runInNewContext("settingsTab='about';renderSettings()",context);
 assert.match(node('#settingsSection').innerHTML,/r0ga1\/Diktator/);
 for(const key of ['gpl','permission','thirdParty','microsoft','privacy']) assert.ok(node('#settingsSection').innerHTML.includes('data-legal=\"'+key+'\"'));
 vm.runInNewContext("settingsTab='general';renderSettings()",context);
 assert.match(node('#settingsSection').innerHTML,/Tastenkürzel/);
 assert.doesNotMatch(node('#settingsSection').innerHTML,/Aufnahme verwerfen|data-capture="cancel"/);
 vm.runInNewContext("settingsTab='display';renderSettings()",context);
 assert.match(node('#settingsSection').innerHTML,/startVolume/);assert.match(node('#settingsSection').innerHTML,/showOverlay/);
 console.log('PASS: hidden boot reaches dashboard/nativeReady; visible polling resumes; hidden idle polling stops; fixed update source and license links render');
})().catch(e=>{console.error(e);process.exitCode=1});
