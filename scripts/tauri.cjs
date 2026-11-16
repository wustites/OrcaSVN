const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const { collectWindowsRelease } = require('./collect-release.cjs')

const root = path.resolve(__dirname, '..')
const args = process.argv.slice(2)
const startedAt = Date.now()
const result = spawnSync(process.execPath, [require.resolve('@tauri-apps/cli/tauri.js'), ...args], {
  cwd: root,
  env: process.env,
  stdio: 'inherit'
})

if (result.error || result.status !== 0) {
  if (result.error) console.error(`Tauri 启动失败：${result.error.message}`)
  process.exit(result.status || 1)
}

// 开发、帮助和调试构建不更新最新发布目录。
const skipCollection = args[0] !== 'build' || args.some(arg =>
  ['--help', '-h', '--version', '-V', '--debug', '-d', '--profile'].includes(arg) || arg.startsWith('--profile=')
)
if (!skipCollection && process.platform === 'win32') {
  try {
    function optionValue(name, shortName) {
      const index = args.findIndex(arg => arg === name || (shortName && arg === shortName))
      if (index >= 0) return args[index + 1]
      return args.find(arg => arg.startsWith(`${name}=`))?.slice(name.length + 1)
    }
    const targetDir = optionValue('--target-dir') || process.env.CARGO_TARGET_DIR || 'target'
    const target = optionValue('--target', '-t') || process.env.CARGO_BUILD_TARGET || ''
    const outputDir = path.resolve(root, 'src-tauri', targetDir, target, 'release')
    const config = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'))
    collectWindowsRelease({ root, outputDir, version: config.version, startedAt, target,
      includeBundles: !args.includes('--no-bundle') })
  } catch (error) {
    console.error(`构建成功，但发布目录更新失败：${error.message}`)
    console.error('请关闭 release 目录中运行的程序，检查文件权限后重新执行构建。原构建产物仍保留在 Cargo 输出目录。')
    process.exitCode = 1
  }
}
