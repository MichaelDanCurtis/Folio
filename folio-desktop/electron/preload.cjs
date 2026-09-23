const {contextBridge,ipcRenderer}=require('electron');
contextBridge.exposeInMainWorld('folio',{
  load:()=>ipcRenderer.invoke('workspace:load'),
  autosave:data=>ipcRenderer.invoke('workspace:autosave',data),
  save:data=>ipcRenderer.invoke('workspace:save',data),
  open:()=>ipcRenderer.invoke('workspace:open'),
  exportFile:(kind,data)=>ipcRenderer.invoke('workspace:export',kind,data),
  connect:()=>ipcRenderer.invoke('agent:connect'),
  login:()=>ipcRenderer.invoke('agent:login'),
  prompt:text=>ipcRenderer.invoke('agent:prompt',text),
  stop:()=>ipcRenderer.invoke('agent:stop'),
  reset:()=>ipcRenderer.invoke('agent:reset'),
  onAgent:callback=>{const handler=(_,event)=>callback(event);ipcRenderer.on('agent:event',handler);return()=>ipcRenderer.removeListener('agent:event',handler);},
  onTool:callback=>{ipcRenderer.on('agent:tool',async(_,msg)=>{try{const value=await callback(msg.name,msg.args);ipcRenderer.send('agent:tool-result',{id:msg.id,value});}catch(e){ipcRenderer.send('agent:tool-result',{id:msg.id,error:e.message});}});}
});
