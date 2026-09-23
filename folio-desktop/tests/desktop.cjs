const {_electron:electron}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const path=require('node:path');
(async()=>{
  const data=await fs.mkdtemp('/private/tmp/folio-desktop-test-');
  const root=path.resolve(__dirname,'..');
  let app;
  async function launch(){app=await electron.launch({args:[root],env:{...process.env,FOLIO_TEST_DATA:data}});const page=await app.firstWindow();await page.locator('.tiptap').waitFor();return page;}
  try{
    let page=await launch();const errors=[];page.on('pageerror',e=>errors.push(e.message));
    assert.match(await page.locator('.tiptap').innerText(),/Good ideas start here/);
    await page.locator('.mode-tabs [data-mode="spreadsheet"]').click();
    assert.equal(await page.locator('[data-cell="B9"]').inputValue(),'20,000');
    await page.locator('[data-cell="B4"]').fill('3000');await page.locator('[data-cell="B4"]').press('Enter');
    assert.equal(await page.locator('[data-cell="B9"]').inputValue(),'20,600');
    await page.locator('#undo').click();assert.equal(await page.locator('[data-cell="B9"]').inputValue(),'20,000');
    await page.locator('#redo').click();assert.equal(await page.locator('[data-cell="B9"]').inputValue(),'20,600');
    await page.locator('#chart-toggle').click();assert.match(await page.locator('#chart-content').innerText(),/Research/);
    await page.locator('.mode-tabs [data-mode="document"]').click();
    await page.locator('.tiptap').click();await page.keyboard.press('Meta+End');await page.keyboard.press('Enter');await page.keyboard.type('Persistence check.');
    await page.waitForFunction(()=>document.querySelector('#save-status').textContent==='Saved on this Mac');
    await page.screenshot({path:path.resolve(root,'../../work/desktop-document.png')});
    await app.close();page=await launch();assert.match(await page.locator('.tiptap').innerText(),/Persistence check/);
    await page.locator('.mode-tabs [data-mode="spreadsheet"]').click();assert.equal(await page.locator('[data-cell="B4"]').inputValue(),'3,000');
    await page.screenshot({path:path.resolve(root,'../../work/desktop-workbook.png')});
    console.log('PASS native editing, recalculation, undo/redo, chart, autosave and relaunch');
    const savedPath=path.join(data,'saved.folio'),csvPath=path.join(data,'sheet.csv'),htmlPath=path.join(data,'document.html');
    await app.evaluate(({dialog},file)=>{dialog.showSaveDialog=async()=>({canceled:false,filePath:file});},savedPath);
    await page.locator('#save').click();await page.waitForFunction(()=>document.querySelector('#toast').textContent==='Saved saved.folio');
    const saved=JSON.parse(await fs.readFile(savedPath,'utf8'));assert.equal(saved.sheets[0].cells.B4,'3000');
    await app.evaluate(({dialog},file)=>{dialog.showSaveDialog=async()=>({canceled:false,filePath:file});},csvPath);
    await page.locator('#export').click();await page.waitForFunction(()=>document.querySelector('#toast').textContent==='Exported sheet.csv');assert.match(await fs.readFile(csvPath,'utf8'),/20600/);
    await page.locator('.mode-tabs [data-mode="document"]').click();
    await app.evaluate(({dialog},file)=>{dialog.showSaveDialog=async()=>({canceled:false,filePath:file});},htmlPath);
    await page.locator('#export').click();await page.waitForFunction(()=>document.querySelector('#toast').textContent==='Exported document.html');assert.match(await fs.readFile(htmlPath,'utf8'),/Persistence check/);
    await app.evaluate(({dialog},file)=>{dialog.showOpenDialog=async()=>({canceled:false,filePaths:[file]});},savedPath);
    await page.locator('#open').click();await page.waitForFunction(()=>document.querySelector('#save-status').textContent==='Saved on this Mac');assert.match(await page.locator('.tiptap').innerText(),/Persistence check/);
    await page.locator('.mode-tabs [data-mode="spreadsheet"]').click();
    console.log('PASS native save/open, HTML export and calculated CSV export');
    if(process.env.FOLIO_LIVE_TEST==='1'){
      await page.locator('#connect').click();await page.waitForFunction(()=>document.querySelector('#connection-label').textContent==='Connected to Codex',{},{timeout:55000});
      console.log('PASS live Codex connection and account detection');
      await page.locator('#prompt').fill('Use read_workspace, then propose_changes to set budget cell B4 to 3100 and append one paragraph saying exactly: Live agent integration verified. Submit one proposal with the current revision. Do not use other tools.');
      await page.locator('#send').click();await page.waitForFunction(()=>document.querySelector('.proposal') || (!document.querySelector('#send').hidden && document.querySelector('.message.assistant')),{},{timeout:120000});assert.equal(await page.locator('.proposal').count(),1,await page.locator('#messages').innerText());
      assert.equal(await page.locator('[data-cell="B4"]').inputValue(),'3,000');
      console.log('PASS live agent read + proposed edit, content unchanged before Apply');
      await page.locator('.proposal .apply').click();assert.equal(await page.locator('[data-cell="B4"]').inputValue(),'3,100');
      await page.locator('.mode-tabs [data-mode="document"]').click();assert.match(await page.locator('.tiptap').innerText(),/Live agent integration verified/);
      await page.locator('#undo').click();assert.doesNotMatch(await page.locator('.tiptap').innerText(),/Live agent integration verified/);
      console.log('PASS live proposal apply and atomic undo across document and sheet');
      await page.waitForFunction(()=>document.querySelector('#send').hidden===false,{},{timeout:60000});
      await page.screenshot({path:path.resolve(root,'../../work/desktop-agent.png')});
    }
    assert.deepEqual(errors,[]);console.log('PASS no renderer errors');
  }finally{if(app)await app.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
