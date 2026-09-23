const { app,BrowserWindow,ipcMain,dialog,shell,Menu }=require('electron');
const fs=require('node:fs/promises');
const path=require('node:path');
const {Agent}=require('./agent.cjs');
app.setName('Folio');
if(process.env.FOLIO_TEST_DATA) app.setPath('userData',process.env.FOLIO_TEST_DATA);
let win,agent,closing=false,saveQueue=Promise.resolve();
const tools=new Map(); let toolId=0;
function encoded(data){if(!data||data.format!=='folio'||data.version!==1||!Array.isArray(data.sheets)||data.document?.type!=='doc')throw Error('Invalid workspace.');const s=JSON.stringify(data,null,2);if(s.length>8_000_000)throw Error('Workspace exceeds the 8 MB prototype limit.');return s;}
async function atomic(file,text){const tmp=file+'.tmp';await fs.writeFile(tmp,text,'utf8');await fs.rename(tmp,file);}
function enqueue(file,text){const job=saveQueue.then(()=>atomic(file,text));saveQueue=job.catch(()=>{});return job;}
app.whenReady().then(async()=>{
  await fs.mkdir(app.getPath('userData'),{recursive:true});
  const cache=path.join(app.getPath('userData'),'workspace.folio');
  const agentDir=path.join(app.getPath('userData'),'agent-workspace');await fs.mkdir(agentDir,{recursive:true});
  agent=new Agent({cwd:agentDir,emit:event=>{if(win&&!win.isDestroyed())win.webContents.send('agent:event',event);},tool:(name,args)=>new Promise((resolve,reject)=>{const id=++toolId;const timer=setTimeout(()=>{tools.delete(id);reject(Error('Editor did not respond.'));},15000);tools.set(id,{resolve,reject,timer});win.webContents.send('agent:tool',{id,name,args});})});
  win=new BrowserWindow({width:1480,height:980,minWidth:1080,minHeight:700,title:'Folio',titleBarStyle:'hiddenInset',backgroundColor:'#f6f5f1',webPreferences:{preload:path.join(__dirname,'preload.cjs'),contextIsolation:true,nodeIntegration:false,sandbox:true}});
  win.webContents.setWindowOpenHandler(()=>({action:'deny'}));
  win.webContents.on('will-navigate',e=>e.preventDefault());
  win.webContents.session.setPermissionRequestHandler((_,__,callback)=>callback(false));
  const allowed=event=>{if(event.sender!==win.webContents||event.senderFrame!==win.webContents.mainFrame)throw Error('Unknown caller.');};
  const handle=(channel,fn)=>ipcMain.handle(channel,(event,...args)=>{allowed(event);return fn(...args);});
  handle('workspace:load',async()=>{try{return JSON.parse(await fs.readFile(cache,'utf8'));}catch(e){if(e.code==='ENOENT')return null;throw Error('The saved workspace could not be read. Your file has been preserved.');}});
  handle('workspace:autosave',data=>enqueue(cache,encoded(data)));
  handle('workspace:save',async data=>{const text=encoded(data);const r=await dialog.showSaveDialog(win,{defaultPath:'Untitled.folio',filters:[{name:'Folio workspace',extensions:['folio']}]});if(r.canceled)return null;await enqueue(r.filePath,text);return path.basename(r.filePath);});
  handle('workspace:open',async()=>{agent.reset();const r=await dialog.showOpenDialog(win,{properties:['openFile'],filters:[{name:'Folio workspace',extensions:['folio']}]});if(r.canceled)return null;const info=await fs.stat(r.filePaths[0]);if(info.size>8_000_000)throw Error('Workspace exceeds 8 MB.');return JSON.parse(await fs.readFile(r.filePaths[0],'utf8'));});
  handle('workspace:export',async(kind,data)=>{if(!['csv','html'].includes(kind)||typeof data!=='string'||data.length>8_000_000)throw Error('Invalid export.');const r=await dialog.showSaveDialog(win,{defaultPath:`Export.${kind}`,filters:[{name:kind.toUpperCase(),extensions:[kind]}]});if(r.canceled)return null;await enqueue(r.filePath,data);return path.basename(r.filePath);});
  handle('agent:connect',()=>agent.connect());
  handle('agent:login',async()=>{const r=await agent.login();if(r.authUrl){const u=new URL(r.authUrl);if(u.protocol!=='https:'||!['auth.openai.com','chatgpt.com','auth.chatgpt.com'].includes(u.hostname))throw Error('Unexpected login URL.');await shell.openExternal(u.href);}return {started:true};});
  handle('agent:prompt',text=>agent.prompt(text));handle('agent:stop',()=>agent.stop());handle('agent:reset',()=>agent.reset());
  ipcMain.on('agent:tool-result',(event,r)=>{allowed(event);const t=tools.get(r.id);if(t){clearTimeout(t.timer);tools.delete(r.id);r.error?t.reject(Error(r.error)):t.resolve(r.value);}});
  Menu.setApplicationMenu(Menu.buildFromTemplate([{label:'Folio',submenu:[{role:'about'},{type:'separator'},{role:'quit'}]},{label:'Edit',submenu:[{role:'undo'},{role:'redo'},{type:'separator'},{role:'cut'},{role:'copy'},{role:'paste'},{role:'selectAll'}]},{label:'View',submenu:[{role:'resetZoom'},{role:'zoomIn'},{role:'zoomOut'},{role:'togglefullscreen'}]}]));
  await win.loadFile(path.join(__dirname,'../dist/index.html'));
  win.on('close',e=>{if(!closing){e.preventDefault();closing=true;saveQueue.finally(()=>win.destroy());}});
});
app.on('window-all-closed',()=>app.quit());app.on('before-quit',()=>agent?.close());
