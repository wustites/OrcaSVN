// Read-only with respect to user workspaces: all SVN operations use a new fixture.
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { pathToFileURL } = require('node:url')
const { spawnSync } = require('node:child_process')

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'orcasvn-encoding-'))
const workspace = path.join(root, 'wc')
const results = []
function run(executable, args, cwd = root, env = process.env) {
  const result = spawnSync(executable, args, { cwd, env, windowsHide: true })
  if (result.error) throw result.error
  results.push({ executable, args, locale: env.LC_ALL || null, status: result.status,
    stdout: result.stdout.toString('utf8'), stderr: result.stderr.toString('utf8'),
    // SVN may emit the system code page; retain original bytes for diagnosis.
    stderrBase64: result.stderr.toString('base64') })
  return result.status
}
try {
  run('svn', ['--version', '--quiet'])
  if (run('svnadmin', ['create', path.join(root, 'repo')]) !== 0) throw new Error('svnadmin create failed')
  if (run('svn', ['checkout', pathToFileURL(path.join(root, 'repo')).href, workspace]) !== 0) throw new Error('checkout failed')
  for (const name of ['ascii.txt', '甲.txt', '中文 空格.txt', '日本語.txt', 'emoji-🐋.txt']) {
    fs.writeFileSync(path.join(workspace, name), 'encoding probe\n')
    run('svn', ['add', '--', name], workspace)
  }
  for (const locale of ['C.UTF-8', '.UTF8', 'zh_CN.UTF-8', 'en_US.UTF-8']) {
    const name = `locale-${results.length}-甲.txt`
    fs.writeFileSync(path.join(workspace, name), 'locale probe\n')
    run('svn', ['add', '--', name], workspace, { ...process.env, LC_ALL: locale })
  }
  console.log(JSON.stringify({ platform: process.platform, release: os.release(),
    note: 'Direct CLI baseline only. windowsHide does not guarantee the app CREATE_NO_WINDOW flags. Run the Rust windows_unicode_arguments regression to verify app behavior.', results }, null, 2))
  if (results.some(result => result.status !== 0)) process.exitCode = 1
} finally {
  if (path.dirname(root) === os.tmpdir() && path.basename(root).startsWith('orcasvn-encoding-')) {
    fs.rmSync(root, { recursive: true, force: true })
  }
}
