const { spawn } = require('node:child_process');
const { once } = require('node:events');
const fs = require('node:fs/promises');
const http = require('node:http');
const net = require('node:net');
const path = require('node:path');

// 固定随机种子，保证演示数据可复现；所有目录、备注和代码均为虚构。
const seed = 20261113;
let randomState = seed;
function randomInt(max) {
  randomState = (Math.imul(randomState, 1664525) + 1013904223) >>> 0;
  return Math.floor(randomState / 0x100000000 * max);
}

const root = path.resolve(__dirname, '..');
const imageDirectory = path.join(root, 'docs', 'images');
const sampleFiles = [
  ['app/src/main/MainActivity.kt', 'modified', '界面优化'],
  ['app/src/main/res/layout/home.xml', 'modified', '界面优化'],
  ['app/src/main/res/values/strings.xml', 'added', '界面优化'],
  ['ui/theme/Theme.kt', 'modified', '主题调整'],
  ['ui/theme/Palette.kt', 'added', '主题调整'],
  ['ui/legacy/old_theme.xml', 'missing', null],
  ['notes/review.md', 'unversioned', null],
].map(([file, status, changelist]) => ({
  path: file, status, status_code: status, changelist,
  prop_status: 'none', locked: false, history: false, switched: false,
}));
const diff = `Index: app/src/main/MainActivity.kt
===================================================================
--- app/src/main/MainActivity.kt\t(revision 247)
+++ app/src/main/MainActivity.kt\t(working copy)
@@ -1,11 +1,15 @@
 package demo.aurora
\x20
 import android.os.Bundle
 import androidx.activity.ComponentActivity
\x20
 class MainActivity : ComponentActivity() {
     override fun onCreate(state: Bundle?) {
         super.onCreate(state)
-        setContent { WelcomeScreen() }
+        setContent {
+            DemoTheme {
+                HomeScreen(title = "Aurora Demo")
+            }
+        }
     }
 }
`;

function createDemoData() {
  const names = ['aurora-app', 'orbit-notes', 'nimbus-ui', 'lumen-player', 'comet-tasks', 'meadow-kit', 'atlas-reader', 'pebble-clock', 'prism-gallery', 'breeze-utils'];
  const notes = ['界面微调', '待评审', '主题样例', '功能草稿', '组件整理', '临时分支'];
  const workspaces = names.map((name, index) => ({
    name,
    path: `C:\\Demo\\Workspaces\\${name}`,
    note: index === 0 ? notes[0] : notes[1 + randomInt(notes.length - 1)],
    favorite: [0, 2, 5].includes(index),
    revision: 247 + randomInt(80),
    latestModified: Date.parse('2026-06-18T10:24:00+08:00') - randomInt(60 * 48) * 60000,
    files: index === 0 ? sampleFiles : sampleFiles.filter(() => randomInt(3) > 0),
  }));
  workspaces[0].revision = 247;
  return { description: '完全虚构的本地截图数据，不对应真实目录或仓库。', seed, workspaces, diff };
}

async function unusedPort() {
  const server = net.createServer();
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
}

function canConnect(url) {
  return new Promise(resolve => {
    const request = http.get(url, response => {
      response.resume();
      resolve(response.statusCode === 200);
    });
    request.setTimeout(1000, () => request.destroy());
    request.on('error', () => resolve(false));
  });
}

async function main() {
  let chromium;
  try { ({ chromium } = require('playwright')); }
  catch { throw new Error('请先安装 Playwright，具体步骤见 docs/screenshot-demo.md。'); }
  const data = createDemoData();
  const port = await unusedPort();
  const origin = `http://127.0.0.1:${port}`;
  // 单独启动仅监听本机的预览服务，不占用现有开发服务。
  const server = spawn(process.execPath, [path.join(root, 'node_modules', 'vite', 'bin', 'vite.js'), '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
    cwd: root, stdio: 'ignore', windowsHide: true,
  });
  let browser;
  try {
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      if (await canConnect(origin)) { ready = true; break; }
      if (server.exitCode !== null) throw new Error('本地预览服务启动失败。');
      await new Promise(resolve => setTimeout(resolve, 200));
    }
    if (!ready) throw new Error('等待本地预览服务超时。');
    browser = await chromium.launch({
      headless: true,
      ...(process.env.SCREENSHOT_BROWSER_CHANNEL ? { channel: process.env.SCREENSHOT_BROWSER_CHANNEL }
        : process.platform === 'win32' ? { channel: 'msedge' } : {}),
    });
    // 不传入用户数据目录，浏览器上下文与桌面程序、本机浏览器完全隔离。
    const context = await browser.newContext({ viewport: { width: 1600, height: 960 }, deviceScaleFactor: 1, locale: 'zh-CN', timezoneId: 'Asia/Shanghai', serviceWorkers: 'block' });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => {
      const url = new URL(route.request().url());
      if (url.origin === origin) return route.continue();
      // 更新检查也返回本地结果，所有外部请求均不发送。
      if (url.hostname === 'api.github.com') return route.fulfill({ json: { tag_name: 'v0.6.4' } });
      return route.abort();
    });
    await page.addInitScript(demo => {
      localStorage.clear();
      const paths = demo.workspaces.map(workspace => workspace.path);
      localStorage.setItem('orcasvn-settings', JSON.stringify({ language: 'zh-CN', theme: 'light', gitignoreEnabled: false }));
      localStorage.setItem('orcasvn-recent-workspaces', JSON.stringify(paths));
      localStorage.setItem('orcasvn-last-workspace', paths[0]);
      localStorage.setItem('orcasvn-favorite-workspaces-v1', JSON.stringify(demo.workspaces.filter(workspace => workspace.favorite).map(workspace => workspace.path)));
      localStorage.setItem('orcasvn-workspace-notes-v1', JSON.stringify(Object.fromEntries(demo.workspaces.map(workspace => [workspace.path, workspace.note]))));
      window.__TAURI_INTERNALS__ = { invoke: async (command, args = {}) => {
        if (command === 'configure_svn_executable') return 'svn';
        if (command === 'launch_workspace_path' || command === 'read_gitignore') return null;
        const workspace = demo.workspaces.find(item => item.path === (args.path || args.workspacePath));
        if (!workspace) throw new Error('截图调用包含非演示目录，已阻止。');
        if (command === 'svn_local_revision') return workspace.revision;
        if (command === 'svn_info') return { path: workspace.path, url: `https://svn.example.test/demo/${workspace.name}/trunk`, repository_root: 'https://svn.example.test/demo', revision: workspace.revision, node_kind: 'dir', schedule: 'normal' };
        if (command === 'workspace_latest_modified') return workspace.latestModified;
        if (command === 'svn_status') return workspace.files;
        if (command === 'svn_diff') return { path: args.file, diff: demo.diff, old_revision: workspace.revision, new_revision: workspace.revision };
        throw new Error('截图环境禁止执行此命令：' + command);
      } };
    }, data);
    await page.goto(origin);
    await page.waitForFunction(count => document.querySelectorAll('.workspace-card').length === count
      && document.querySelector('.workspace-list')?.getAttribute('aria-busy') === 'false'
      && document.querySelectorAll('.changelist-header').length === 3, data.workspaces.length);
    await page.locator('.file-path').filter({ hasText: 'app/src/main/MainActivity.kt' }).click();
    await page.locator('.diff-code').filter({ hasText: 'HomeScreen' }).waitFor();
    await page.evaluate(() => document.fonts.ready);
    await page.mouse.move(1550, 930);
    if (errors.length) throw new Error('演示页面出现错误：' + errors.join('；'));
    await fs.mkdir(imageDirectory, { recursive: true });
    await page.screenshot({ path: path.join(imageDirectory, 'orcasvn-workspace.png'), animations: 'disabled' });
    await page.getByRole('tab', { name: /^收藏/ }).click();
    if (await page.locator('.workspace-card').count() !== 3) throw new Error('收藏演示列表未就绪。');
    await page.mouse.move(1550, 930);
    await page.screenshot({ path: path.join(imageDirectory, 'orcasvn-workspace-favorites.png'), animations: 'disabled' });
    await page.getByRole('tab', { name: /^全部/ }).click();
    await page.evaluate(async () => {
      const { useSettings } = await import('/src/composables/useSettings.ts');
      useSettings().updateSettings({ theme: 'dark' });
    });
    await page.waitForFunction(() => document.documentElement.classList.contains('dark'));
    await page.screenshot({ path: path.join(imageDirectory, 'orcasvn-workspace-dark.png'), animations: 'disabled' });
    await fs.writeFile(path.join(imageDirectory, 'workspace-demo-data.json'), JSON.stringify(data, null, 2) + '\n', 'utf8');
    if (errors.length) throw new Error('演示页面出现错误：' + errors.join('；'));
    console.log('已生成 10 个虚构工作区、3 个收藏及浅色／收藏／深色截图；未访问真实 SVN 或工作区目录。');
  } finally {
    if (browser) await browser.close();
    if (server.exitCode === null) {
      const stopped = once(server, 'exit');
      server.kill();
      await stopped;
    }
  }
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
