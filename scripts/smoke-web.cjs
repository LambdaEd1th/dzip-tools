#!/usr/bin/env node
// Run against a served build-web.sh bundle. Requires Node, an installed browser,
// Info-ZIP's unzip on PATH, and Playwright (only the test depends on it):
// npm install --prefix /tmp/dzip-browser-test playwright
// /tmp/dzip-browser-test/node_modules/.bin/playwright install chromium
// NODE_PATH=/tmp/dzip-browser-test/node_modules node scripts/smoke-web.cjs http://localhost:8000/
// Append --i18n, --logs or --controls to run just that feature's tests.
// Optionally set DZIP_CHROME_CHANNEL=chrome or DZIP_SMOKE_OUT=/path/to/artifacts.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { chromium } = require('playwright');

const fixture = path.resolve(__dirname, '../vendor/dzip/test_data/native');
const out = process.env.DZIP_SMOKE_OUT || fs.mkdtempSync(path.join(os.tmpdir(), 'dzip-web-smoke-'));
fs.mkdirSync(out, { recursive: true });
const url = process.argv[2] || 'http://localhost:8000/';
const btn = (p, name) => p.getByRole('button', { name, exact: typeof name === 'string' });
const box = (p, name) => p.getByRole('checkbox', { name, exact: true });
const input = (p, name) => p.getByRole('textbox', { name, exact: true });
const file = (name, buffer) => ({ name, mimeType: 'application/octet-stream', buffer });

async function choose(p, name, files) {
    const [picker] = await Promise.all([p.waitForEvent('filechooser'), btn(p,name).click()]);
    await picker.setFiles(files);
}
async function download(p, name, artifact) {
    const [d] = await Promise.all([p.waitForEvent('download'), btn(p,name).click()]);
    await d.saveAs(path.join(out,artifact));
    assert.equal(await d.failure(), null);
    return [d.suggestedFilename(),path.join(out,artifact)];
}
async function drop(p, files) {
    // Drive the browser's real drop listener, File API, worker and exporter.
    await p.evaluate((items) => {
        const transfer = new DataTransfer();
        for (const {name, data} of items)
            transfer.items.add(new File([Uint8Array.from(atob(data), c => c.charCodeAt(0))], name));
        for (const type of ['dragover','drop']) document.body.dispatchEvent(
            new DragEvent(type,{dataTransfer:transfer,bubbles:true,cancelable:true}));
    },files.map(({name,buffer})=>({name,data:buffer.toString('base64')})));
}
// Independent ZIP reader: check actual archive bytes and paths, not just toasts.
const paths = zip => execFileSync('unzip',['-Z1',zip],{encoding:'utf8'}).trim().split('\n').sort();
const bytes = (zip, name) => execFileSync('unzip',['-p',zip,name]);
async function shot(p, name) {
    const dismiss=btn(p,/Dismiss notification|关闭通知|关闭提示/);
    if (await dismiss.count()) await dismiss.first().click();
    await p.mouse.move(p.viewportSize().width-1,0);
    await p.screenshot({path:path.join(out,name+'.png'),animations:'disabled'});
}
function attachErrors(page,errors) {
    page.setDefaultTimeout(15000);
    page.on('pageerror',error => errors.push(String(error)));
    page.on('response',r => {if(r.status() >= 400) errors.push(r.status()+' '+r.url());});
}

async function localization(browser, errors) {
    const context = await browser.newContext({locale: 'fr-CA'});
    const p = await context.newPage();
    attachErrors(p, errors);
    try {
        // fr-CA uses the French catalogue. An existing preference from the
        // regional-code catalogues must still take precedence over the device.
        await p.goto(url);
        await btn(p, 'Ouvrir les paramètres').click();
        const picker = p.getByRole('dialog').getByRole('combobox');
        await picker.press('ArrowDown');
        await p.getByRole('listbox').waitFor();
        await p.keyboard.press('Escape');
        await p.getByRole('listbox').waitFor({state:'hidden'});
        assert.equal(await p.evaluate(() => localStorage.getItem('dzip-archive-locale')),null);
        // Explicitly selecting the system's current language still pins it.
        await picker.click();
        await p.getByRole('option', {name:'Français', exact:true}).click();
        await p.waitForFunction(() => localStorage.getItem('dzip-archive-locale')?.split('-')[0] === 'fr');
        await p.evaluate(() => localStorage.setItem('dzip-archive-locale', 'es-ES'));
        await p.reload();
        await choose(p, 'Elegir archivos .dz', path.join(fixture, 'tiny.dz'));
        await p.getByText('tiny.dz abierto', {exact: true}).waitFor();
        await btn(p, 'Abrir configuración').click();

        // Use the actual language picker. Checking the open dialog also
        // catches applications that only apply new translations on reload.
        const languages = [
            ['English', 'Settings'], ['Español', 'Configuración'],
            ['Français', 'Paramètres'], ['简体中文', '设置'], ['Русский', 'Настройки'],
        ];
        for (const [name, settings] of languages) {
            await picker.click();
            const menu = p.getByRole('listbox');
            await menu.getByRole('option', {name, exact: true}).click();
            await p.getByRole('dialog', {name: settings, exact: true}).waitFor();
        }
        await p.reload();
        await btn(p, 'Открыть настройки').waitFor();
    } catch (error) {
        fs.writeFileSync(path.join(out,'language-failure.txt'),await p.locator('body').ariaSnapshot().catch(()=>''));
        throw error;
    } finally { await context.close(); }
}

async function logging(browser, errors) {
    const context = await browser.newContext({locale: 'en-US'});
    const p = await context.newPage();
    attachErrors(p, errors);
    const consoleLines = [];
    p.on('console', message => consoleLines.push({type: message.type(), text: message.text()}));
    const dialog = p.getByRole('dialog', {name: 'Application logs', exact: true});
    const content = input(dialog, 'Application log content');
    async function openLogs() {
        await btn(p, 'Open settings').click();
        await btn(p, 'View application logs').click();
    }
    async function closeLogs() {
        await btn(dialog, 'Close logs').click();
        await btn(p, 'Close settings').click();
    }
    try {
        await p.goto(url);
        const invalid = 'The dropped content does not contain a .dz or .dzip main archive';
        await choose(p, 'Choose .dz files', file('no-archive.txt', Buffer.from('not an archive')));
        await p.getByRole('alert',{name:invalid,exact:true}).waitFor();
        await choose(p, 'Choose .dz files', path.join(fixture, 'tiny.dz'));
        await btn(p, 'Open folder local').waitFor();
        const notice = p.getByRole('alert',{name:'Opened tiny.dz',exact:true});
        await notice.waitFor();
        assert.equal(await p.getByRole('alert').count(),1,'only the latest notice remains');
        await btn(notice,'Dismiss notification').click();
        await notice.waitFor({state:'detached'});
        await openLogs();
        const displayed = await content.inputValue();
        assert.match(displayed, /Dzip Archive/i, 'startup message is visible');
        assert.match(displayed, /Opened tiny\.dz/, 'archive activity is visible');
        assert.ok(displayed.includes(invalid), 'failed opens are logged too');
        assert.equal(await content.isEditable(), false);
        const [name, exported] = await download(p, 'Export log', 'session.log');
        assert.equal(name, 'dzip-archive.log');
        assert.equal(fs.readFileSync(exported, 'utf8'), displayed);

        await btn(dialog, 'Clear').click();
        assert.equal(await content.inputValue(), '');
        await closeLogs();
        await btn(p, 'Close archive').click();
        await btn(p, 'Choose .dz files').waitFor();
        await openLogs();
        const afterClear = await content.inputValue();
        assert.match(afterClear, /Closed tiny\.dz/);
        assert.doesNotMatch(afterClear, /Opened tiny\.dz/, 'cleared entries stay cleared');
        for (const text of ['Opened tiny.dz', 'Closed tiny.dz']) {
            assert.ok(consoleLines.some(line => line.text.includes(text)), `missing from browser console: ${text}`);
        }
        assert.ok(consoleLines.some(line => line.type === 'error' && line.text.includes(invalid)),
            'NSH forwards failed operations to the browser at error severity');
    } finally { await context.close(); }
}

async function controls(browser, errors) {
    const context = await browser.newContext({locale:'en-US',colorScheme:'light',viewport:{width:1280,height:900}});
    const p = await context.newPage();
    attachErrors(p,errors);
    const radio = (group,name) => group.getByRole('radio',{name,exact:true});
    const focused = locator => locator.evaluate(el => el === document.activeElement);
    try {
        await p.goto(url);
        const opener = btn(p,'Open settings');
        await opener.press('Enter');
        // On a palette change NSH briefly keeps the departing, inert modal
        // mounted. Interact with the active layer, as a keyboard user does.
        const settings = p.getByRole('dialog',{name:'Settings',exact:true}).and(p.locator(':not([inert])'));
        const themes = settings.getByRole('radiogroup',{name:'Appearance mode'});
        await radio(themes,'System').press('ArrowRight');
        await themes.getByRole('radio',{name:'Light',checked:true}).waitFor();
        await p.keyboard.press('ArrowRight');
        await themes.getByRole('radio',{name:'Dark',checked:true}).waitFor();
        assert.equal(await p.evaluate(() => localStorage.getItem('dzip-archive-theme')),'dark');

        const openLogs = btn(settings,'View application logs');
        await openLogs.press('Enter');
        const logs = p.getByRole('dialog',{name:'Application logs',exact:true});
        await logs.waitFor();
        assert.equal(await logs.getByRole('button',{name:'',exact:true}).count(),0,
            'the pointer-only backdrop is not an unnamed button for screen readers');
        // The host leaves the workspace and Settings mounted, but inert.
        await p.waitForFunction(() => ['Open settings','View application logs'].every(name =>
            document.querySelector(`[aria-label="${name}"]`)?.closest('[inert]')));
        await p.keyboard.press('Escape');
        await logs.waitFor({state:'hidden'});
        await openLogs.waitFor();
        assert.ok(await focused(openLogs),'closing logs returns focus to its opener in Settings');
        await p.keyboard.press('Escape');
        await settings.waitFor({state:'hidden'});
        assert.ok(await focused(opener),'closing Settings returns focus to the toolbar');
        await opener.click();
        await settings.waitFor();
        await p.mouse.click(8,8); // the backdrop, outside the panel
        await settings.waitFor({state:'hidden'});

        await btn(p,'Create new archive').click();
        const codecs = p.getByRole('radiogroup',{name:'Default algorithm for new files'});
        await radio(codecs,'DZ').press('ArrowRight');
        await codecs.getByRole('radio',{name:'Zlib',checked:true}).waitFor();
        await p.keyboard.press('ArrowRight'); // cross the card grid's row boundary
        await codecs.getByRole('radio',{name:'Bzip',checked:true}).waitFor();
        assert.ok(await focused(radio(codecs,'Bzip')));
    } catch (error) {
        fs.writeFileSync(path.join(out,'controls-failure.txt'),await p.locator('body').ariaSnapshot().catch(()=>''));
        throw error;
    } finally {await context.close();}
}

async function desktop(browser,errors) {
    const context=await browser.newContext({locale:'en-US',colorScheme:'light',viewport:{width:1280,height:900}});
    const p=await context.newPage();
    attachErrors(p,errors);
    try {
        await p.goto(url);
        await choose(p,'Choose .dz files',path.join(fixture,'tiny.dz'));
        const open=btn(p,'Open folder local');
        await open.waitFor();
        await shot(p,'desktop-archive');
        await open.click();
        await btn(p,'local/empty.bin').click();
        await p.getByText('File details',{exact:true}).waitFor();
        await shot(p,'desktop-file');
        await box(p,'Select local/one.bin').click();
        const [selectedName,selected]=await download(p,'Extract selected (1)','selected.zip');
        assert.equal(selectedName,'tiny-extracted.zip');
        assert.deepEqual(paths(selected),['one.bin']);
        assert.deepEqual(bytes(selected,'one.bin'),fs.readFileSync(path.join(fixture,'corpus/local/one.bin')));
        await btn(p,'Root').click();
        const folderBox = box(p,'Select all files in folder local');
        assert.equal(await folderBox.getAttribute('aria-checked'),'mixed');
        await input(p,'Search archive contents').fill('TwO');
        await btn(p,'local/two.bin').waitFor();
        assert.equal(await btn(p,'local/one.bin').count(),0);
        await box(p,'Select all visible entries').click();
        await btn(p,'Extract selected (2)').waitFor();
        await btn(p,'Clear search').click();
        await folderBox.press('Space'); // a partially selected folder becomes fully selected
        await btn(p,'Extract selected (3)').waitFor();
        await p.getByRole('checkbox',{name:'Select all files in folder local',checked:true}).waitFor();
        await folderBox.press('Space');
        await p.getByRole('checkbox',{name:'Select all files in folder local',checked:false}).waitFor();
        await folderBox.click();
        await btn(p,'Clear').click();
        await btn(p,'Edit archive').click();
        await input(p,'Archive name').fill('smoke-rebuilt.dz');
        await btn(p,'Open folder local').click();
        const payload=Buffer.from('Dzip compressed round-trip — 0123456789\n'.repeat(100));
        await choose(p,'Add files',file('added.txt',payload));
        const rename=input(p,'Rename local/added.txt');
        await rename.fill('renamed.txt');
        await rename.press('Tab'); // commit the rename on blur
        // Select a non-default codec and put only the new entry on a second volume.
        await p.getByLabel(/^Set compression for local\/renamed\.txt/).click();
        await p.getByRole('option',{name:'Zlib',exact:true}).click();
        await p.getByLabel(/^Set the target volume for local\/renamed\.txt/).click();
        await p.getByRole('option',{name:'New volume 1',exact:true}).click();
        const [splitName,split]=await download(p,'Save archive','rebuilt-volumes.zip');
        assert.equal(splitName,'smoke-rebuilt-volumes.zip');
        assert.deepEqual(paths(split),['smoke-rebuilt.001','smoke-rebuilt.dz']);
        await btn(p,'Close archive').click();
        await btn(p,'Choose .dz files').waitFor();
        // Reverse the order deliberately: a numeric volume is not the main archive.
        await drop(p,paths(split).map(name=>file(name,bytes(split,name))));
        await btn(p,'Open folder local').click();
        const reopenedRow=btn(p,'local/renamed.txt');
        await reopenedRow.click();
        const metadata=await reopenedRow.innerText();
        assert.match(metadata,/\bZLIB\b/);
        assert.match(metadata,/\bVolume 1\b/);
        await p.getByText('File details',{exact:true}).waitFor();
        const [extractName,extracted]=await download(p,'Extract all','rebuilt-extracted.zip');
        assert.equal(extractName,'smoke-rebuilt-extracted.zip');
        assert.deepEqual(paths(extracted),['local/empty.bin','local/one.bin','local/renamed.txt','local/two.bin']);
        assert.deepEqual(bytes(extracted,'local/renamed.txt'),payload);
        for (const f of ['empty.bin','one.bin','two.bin'])
            assert.deepEqual(bytes(extracted,'local/'+f),fs.readFileSync(path.join(fixture,'corpus/local',f)));

        await btn(p,'Close archive').click();
        await btn(p,'Create new archive').click();
        const randomAccess = checked => p.getByRole('switch',{name:'Random access',exact:true,checked});
        await randomAccess(false).click();
        await randomAccess(true).press('Space');
        await randomAccess(false).waitFor();
        const commonBuffer = p.getByRole('switch',{name:'Common buffer',exact:true});
        await p.getByRole('radio',{name:'Zlib',exact:true}).click();
        assert.ok(await commonBuffer.isDisabled(),'common buffer is disabled without a DZ entry');
        await p.getByRole('radio',{name:'DZ',exact:true}).click();
        assert.ok(await commonBuffer.isEnabled(),'selecting DZ re-enables its settings');
        await drop(p,[file('new.txt',payload)]);
        await input(p,'Archive name').fill('fresh.dz');
        await input(p,'Rename new.txt').waitFor();
        const [freshName,fresh]=await download(p,'Create and save','fresh.dz');
        assert.equal(freshName,'fresh.dz');
        await btn(p,'Close archive').click();
        await choose(p,'Choose .dz files',fresh);
        await btn(p,'new.txt').waitFor();
        const [,freshExport]=await download(p,'Extract all','fresh-extracted.zip');
        assert.deepEqual(paths(freshExport),['new.txt']);
        assert.deepEqual(bytes(freshExport,'new.txt'),payload);
    } catch(error) {
        await p.screenshot({path:path.join(out,'desktop-failure.png')}).catch(()=>{});
        fs.writeFileSync(path.join(out,'desktop-failure.txt'),await p.locator('body').ariaSnapshot().catch(()=>''));
        throw error;
    } finally {await context.close();}
}

async function phone(browser,errors) {
    const context=await browser.newContext({locale:'zh-CN',colorScheme:'dark',viewport:{width:390,height:844},hasTouch:true,isMobile:true});
    const p=await context.newPage();
    attachErrors(p,errors);
    try {
        await p.goto(url);
        await btn(p,'打开设置').tap();
        const settings = p.getByRole('dialog',{name:'设置',exact:true});
        for (const [name, href] of [
            ['GitHub','https://github.com/LambdaEd1th/dzip-tools'],
            ['LambdaEd1th','https://space.bilibili.com/8217621'],
        ]) assert.equal(await settings.getByRole('link',{name,exact:true}).getAttribute('href'),href);
        await settings.getByRole('combobox').tap();
        await p.getByRole('option',{name:'English',exact:true}).tap();
        const translated = p.getByRole('dialog',{name:'Settings',exact:true});
        await translated.getByRole('combobox').tap();
        await p.getByRole('option',{name:'简体中文',exact:true}).tap();
        await btn(p,'关闭设置').tap();
        await dropAfterLoad();
        await shot(p,'phone-archive');
        await btn(p,/打开.*local/).click();
        await btn(p,'local/empty.bin').click();
        await p.getByText('文件详情',{exact:true}).waitFor();
        await shot(p,'phone-file');
    } catch(error) {
        fs.writeFileSync(path.join(out,'phone-failure.txt'),await p.locator('body').ariaSnapshot().catch(()=>''));
        throw error;
    } finally {await context.close();}
    async function dropAfterLoad() {
        await btn(p,/选择.*\.dz/).waitFor();
        await drop(p,[file('tiny.dz',fs.readFileSync(path.join(fixture,'tiny.dz')))]);
        await btn(p,/打开.*local/).waitFor();
    }
}

(async()=>{
    console.log('Browser smoke artifacts:',out);
    const browser=await chromium.launch({headless:true,channel:process.env.DZIP_CHROME_CHANNEL||undefined});
    try {
        const errors=[];
        const i18n = process.argv.includes('--i18n');
        const logs = process.argv.includes('--logs');
        const checkControls = process.argv.includes('--controls');
        const all = !i18n && !logs && !checkControls;
        if (all || i18n) {
            await localization(browser, errors);
            console.log('Passed: language negotiation, existing preferences, translated arguments, five live languages and reload.');
        }
        if (all || logs) {
            await logging(browser, errors);
            console.log('Passed: session log display, export, clear, new events and browser console output.');
        }
        if (all || checkControls) {
            await controls(browser, errors);
            console.log('Passed: keyboard radio selection, modal dismissal order, focus restoration and backdrop clicks.');
        }
        if (all) {
            await desktop(browser,errors);
            await phone(browser,errors);
            console.log('Passed: desktop/phone import, selection, search, extraction, edit/rebuild, split-volume reimport and new archive round trip.');
        }
        assert.deepEqual(errors,[]);
    } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
