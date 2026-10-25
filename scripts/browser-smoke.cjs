// Run after `trunk build`. Requires Playwright (npm install --no-save playwright).
// Optional: PLAYWRIGHT_MODULE, CHROME_PATH, SMOKE_OUTPUT environment variables.
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..', process.env.SMOKE_DIST || 'dist');
const output = process.env.SMOKE_OUTPUT || '/tmp/formcraft-smoke';
fs.mkdirSync(output, { recursive: true });
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname === '/favicon.ico') { res.writeHead(204).end(); return; }
  const file = path.resolve(root, '.' + decodeURIComponent(url.pathname === '/' ? '/index.html' : url.pathname));
  if (!file.startsWith(root + path.sep)) { res.writeHead(403).end(); return; }
  fs.readFile(file, (error, data) => {
    if (error) { res.writeHead(404).end(); return; }
    res.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : file.endsWith('.html') ? 'text/html' : 'application/octet-stream');
    res.end(data);
  });
});
(async () => {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH,
    headless: true,
    args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--enable-webgl'],
  });
  try {
    const page = await browser.newPage({ viewport: { width: 1360, height: 900 }, deviceScaleFactor: 1 });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => {
      console.log(`[browser ${message.type()}] ${message.text()}`);
      if (message.type() === 'error' || /%cERROR%c/.test(message.text())) errors.push(message.text());
    });
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.waitForSelector('#formcraft[data-ready]', { timeout: 120000 });
    await page.waitForSelector('#loading', { state: 'detached' });
    await page.waitForTimeout(1500);
    await page.screenshot({ path: path.join(output, '01-empty.png') });
    const chooser = page.waitForEvent('filechooser');
    await page.mouse.click(1265, 30);
    await (await chooser).setFiles(path.resolve(__dirname, '../tests/fixtures/assembly.glb'));
    await page.getByRole('button', { name: 'Ok', exact: true }).click();
    await page.waitForTimeout(4000);
    await page.screenshot({ path: path.join(output, '02-imported.png') });
    const clip = { x: 450, y: 260, width: 500, height: 360 };
    const before = await page.screenshot({ clip });
    await page.mouse.move(710, 450);
    await page.mouse.down();
    await page.mouse.move(840, 510, { steps: 15 });
    await page.mouse.up();
    await page.waitForTimeout(1000);
    const after = await page.screenshot({ clip });
    assert(!before.equals(after), 'Orbit must visibly change the viewport');
    await page.mouse.wheel(0, -250);
    await page.waitForTimeout(1000);
    const zoomed = await page.screenshot({ clip });
    assert(!after.equals(zoomed), 'Zoom must visibly change the viewport');
    await page.screenshot({ path: path.join(output, '03-orbit-zoom.png') });
    await page.mouse.click(524, 85);
    await page.waitForTimeout(1000);
    await page.mouse.click(1198, 303);
    await page.waitForTimeout(2500);
    await page.screenshot({ path: path.join(output, '04-orthographic.png') });
    // Expand the scene and assembly, then select the named cube subtree.
    await page.mouse.click(34, 173);
    await page.waitForTimeout(1500);
    await page.mouse.click(52, 211);
    await page.waitForTimeout(1500);
    await page.mouse.click(120, 250);
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(output, '05-hierarchy.png') });
    assert.deepEqual(errors, [], 'Browser must have no JavaScript or engine errors');
    console.log(`Browser smoke passed. Screenshots: ${output}`);
  } finally { await browser.close(); server.close(); }
})().catch(error => { console.error(error); server.close(); process.exitCode = 1; });
