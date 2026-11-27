const fs = require('node:fs')
const path = require('node:path')

// Tauri uses RFC 7396: objects merge recursively, arrays replace, null deletes.
function mergePatch(base, patch) {
  if (!patch || typeof patch !== 'object' || Array.isArray(patch)) return patch
  const result = base && typeof base === 'object' && !Array.isArray(base) ? { ...base } : {}
  for (const [key, value] of Object.entries(patch)) {
    if (value === null) delete result[key]
    else Object.defineProperty(result, key, { value: mergePatch(result[key], value), enumerable: true, writable: true, configurable: true })
  }
  return result
}

function resolveReleaseConfig(root, rawArgs, env = process.env) {
  const args = rawArgs.slice(0, rawArgs.indexOf('--') < 0 ? rawArgs.length : rawArgs.indexOf('--'))
  const configDir = path.join(root, 'src-tauri')
  const read = file => JSON.parse(fs.readFileSync(file, 'utf8'))
  let config = read(path.join(configDir, 'tauri.conf.json'))
  for (const name of ['tauri.windows.conf.json5', 'Tauri.windows.toml']) {
    if (fs.existsSync(path.join(configDir, name))) throw new Error(`发布汇总暂不支持配置格式：${name}`)
  }
  const platformConfig = path.join(configDir, 'tauri.windows.conf.json')
  if (fs.existsSync(platformConfig)) config = mergePatch(config, read(platformConfig))
  if (env.TAURI_CONFIG) throw new Error('发布汇总不支持外部 TAURI_CONFIG，请使用 --config JSON')
  let bundles
  for (let i = 1; i < args.length; i++) {
    const arg = args[i]
    if (arg === '--config' || arg === '-c' || arg.startsWith('--config=') || arg.startsWith('-c=')) {
      const value = arg.includes('=') ? arg.slice(arg.indexOf('=') + 1) : args[++i]
      if (!value) throw new Error('--config 缺少值')
      let patch
      if (value.trimStart().startsWith('{')) patch = JSON.parse(value)
      else {
        if (path.extname(value) !== '.json') throw new Error('发布汇总仅支持 JSON 配置文件或内联 JSON')
        patch = read(path.resolve(root, value))
      }
      config = mergePatch(config, patch)
    } else if (arg === '--bundles' || arg === '-b' || arg.startsWith('--bundles=') || arg.startsWith('-b=')) {
      bundles = []
      if (arg.includes('=')) bundles.push(...arg.slice(arg.indexOf('=') + 1).split(','))
      while (i + 1 < args.length && !args[i + 1].startsWith('-')) bundles.push(...args[++i].split(','))
    }
  }
  if (config.mainBinaryName && config.mainBinaryName !== 'OrcaSVN') throw new Error('发布汇总不支持 mainBinaryName 覆盖')
  if (config.productName !== 'OrcaSVN') throw new Error('发布汇总不支持 productName 覆盖')
  let expectedBundles = args.includes('--no-bundle') || (!bundles && !config.bundle?.active)
    ? [] : bundles ?? config.bundle?.targets ?? 'all'
  if (expectedBundles === 'all') expectedBundles = ['nsis', 'msi']
  if (!Array.isArray(expectedBundles)) expectedBundles = [expectedBundles]
  if (expectedBundles.some(kind => !['nsis', 'msi'].includes(kind))) throw new Error('发布汇总遇到不支持的 bundle 格式')
  return { version: config.version, expectedBundles: [...new Set(expectedBundles)] }
}

module.exports = { resolveReleaseConfig, mergePatch }
