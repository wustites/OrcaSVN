const { spawnSync } = require('node:child_process')
if (process.platform === 'win32') {
  const rawArgs = process.argv.slice(2)
  const args = rawArgs.slice(0, rawArgs.indexOf('--') < 0 ? rawArgs.length : rawArgs.indexOf('--'))
  const targetIndex = args.findIndex(arg => arg === '--target' || arg === '-t')
  const target = (targetIndex >= 0 ? args[targetIndex + 1] : args.find(arg => arg.startsWith('--target='))?.slice(9)) || process.env.CARGO_BUILD_TARGET
  const arch = target ? ({ x86_64: 'x64', aarch64: 'arm64', i686: 'ia32' })[target.split('-')[0]] : process.arch
  if (!['x64', 'arm64', 'ia32'].includes(arch)) throw new Error(`Unsupported SVN runtime architecture: ${arch}`)
  const result = spawnSync('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
    '-File', require('node:path').join(__dirname, 'prepare-svn.ps1'), '-Architecture', arch], { stdio: 'inherit', windowsHide: true })
  if (result.error) throw result.error
  process.exitCode = result.status ?? 1
}
