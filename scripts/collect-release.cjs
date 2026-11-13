const fs = require('node:fs')
const path = require('node:path')
const crypto = require('node:crypto')
const { spawnSync } = require('node:child_process')

const manifestName = 'release-manifest.json'

function readManagedNames(releaseDir) {
  const manifestPath = path.join(releaseDir, manifestName)
  if (!fs.existsSync(manifestPath)) return new Set()
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'))
  if (manifest.generatedBy !== 'OrcaSVN/collect-release') {
    throw new Error('release 目录中的发布清单不属于本项目，已停止覆盖')
  }
  const names = manifest.files.map(file => file.name)
  if (names.some(name => typeof name !== 'string' || path.basename(name) !== name || ['.', '..'].includes(name))) {
    throw new Error('发布清单包含不安全的文件路径，已停止覆盖')
  }
  return new Set([...names, manifestName])
}

function collectWindowsRelease({ root, outputDir, version, startedAt, target = '', includeBundles = true }) {
  root = path.resolve(root)
  const releaseDir = path.join(root, 'release')
  const sourceExe = path.join(outputDir, 'OrcaSVN.exe')
  if (!fs.statSync(sourceExe).isFile()) throw new Error(`没有找到本次构建程序：${sourceExe}`)
  if (!/^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][\w.-]+)?$/.test(version)) throw new Error('发布版本号无效')
  const architecture = target ? target.split('-')[0].replace('x86_64', 'x64').replace('aarch64', 'arm64') : process.arch
  const portableName = `OrcaSVN-${version}-windows-${architecture}-portable.zip`
  const sources = [{ source: sourceExe, name: 'OrcaSVN.exe' }, { source: path.join(root, 'LICENSE'), name: 'LICENSE' }]
  if (includeBundles) {
    for (const kind of ['nsis', 'msi']) {
      const directory = path.join(outputDir, 'bundle', kind)
      if (!fs.existsSync(directory)) continue
      for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        if (!entry.isFile() || !entry.name.startsWith(`OrcaSVN_${version}_`) || !/\.(exe|msi)$/i.test(entry.name)) continue
        const source = path.join(directory, entry.name)
        // 只收集本次打包更新的安装程序，避免混入旧版本或上一次未选择的格式。
        if (fs.statSync(source).mtimeMs >= startedAt - 2000) sources.push({ source, name: entry.name })
      }
    }
  }
  const generatedNames = new Set([...sources.map(file => file.name), '使用说明.txt', portableName, manifestName])
  const oldManagedNames = readManagedNames(releaseDir)
  const extras = fs.existsSync(releaseDir)
    ? fs.readdirSync(releaseDir).filter(name => !oldManagedNames.has(name)) : []
  if (extras.some(name => generatedNames.has(name))) {
    throw new Error('release 目录有与产物同名的非托管文件，请先移走这些文件，避免覆盖用户数据')
  }
  const stage = fs.mkdtempSync(path.join(root, '.release-stage-'))
  const backup = path.join(root, `.release-previous-${Date.now()}-${crypto.randomBytes(4).toString('hex')}`)
  let previousMoved = false
  let published = false
  try {
    for (const entry of sources) fs.copyFileSync(entry.source, path.join(stage, entry.name))
    fs.writeFileSync(path.join(stage, '使用说明.txt'), [
      `OrcaSVN ${version} Windows 版`, '',
      '免安装运行：双击 OrcaSVN.exe；需要 Windows WebView2 Runtime 和 SVN 命令行工具。',
      '更新前请先退出旧版 OrcaSVN。', '',
      '工作区支持通过资源管理器、Android Studio、IntelliJ IDEA、VS Code 或终端打开目录。',
      '非标准安装位置可在打开方式的设置中选择编辑器程序。',
      '工作区列表支持“全部 / 收藏”页签，点击目录右侧星标可收藏或取消收藏，收藏记录保存在本机。',
      '文件状态按照 SVN changelist 分组，当前筛选结果中的空分组不显示。', '',
      '本目录由发布构建自动更新；release-manifest.json 记录本次构建时间及文件校验值。',
      '自行放入的其他文件会保留；不要修改自动生成的发布清单。', ''
    ].join('\r\n'), 'utf8')
    const portableFolder = path.join(stage, `OrcaSVN-${version}-windows-${architecture}`)
    fs.mkdirSync(portableFolder)
    for (const name of ['OrcaSVN.exe', 'LICENSE', '使用说明.txt']) {
      fs.copyFileSync(path.join(stage, name), path.join(portableFolder, name))
    }
    // 通过环境变量传递路径，避免 PowerShell 字符串拼接导致引号或命令注入问题。
    const zip = spawnSync('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command',
      '$ErrorActionPreference="Stop"; Compress-Archive -LiteralPath $env:ORCASVN_PORTABLE_SOURCE -DestinationPath $env:ORCASVN_PORTABLE_ZIP -CompressionLevel Optimal'
    ], { windowsHide: true, stdio: 'pipe', encoding: 'utf8', env: { ...process.env,
      ORCASVN_PORTABLE_SOURCE: portableFolder, ORCASVN_PORTABLE_ZIP: path.join(stage, portableName) } })
    if (zip.error || zip.status !== 0) throw new Error(`免安装包生成失败：${zip.error?.message || zip.stderr}`)
    // 只清理本次创建且已验证在暂存目录内的临时文件夹。
    if (path.dirname(portableFolder) !== stage) throw new Error('临时目录越界，已停止清理')
    fs.rmSync(portableFolder, { recursive: true })
    const files = [...generatedNames].filter(name => name !== manifestName).map(name => {
      const content = fs.readFileSync(path.join(stage, name))
      return { name, bytes: content.length, sha256: crypto.createHash('sha256').update(content).digest('hex') }
    })
    fs.writeFileSync(path.join(stage, manifestName), JSON.stringify({
      generatedBy: 'OrcaSVN/collect-release', version, builtAt: new Date().toISOString(),
      target: target || `windows-${process.arch}`, files
    }, null, 2) + '\n', 'utf8')
    for (const name of extras) fs.cpSync(path.join(releaseDir, name), path.join(stage, name), { recursive: true })
    if (fs.existsSync(releaseDir)) {
      fs.renameSync(releaseDir, backup)
      previousMoved = true
    }
    fs.renameSync(stage, releaseDir)
    published = true
    if (previousMoved) {
      try {
        if (path.dirname(backup) !== root) throw new Error('旧发布目录越界')
        fs.rmSync(backup, { recursive: true })
      } catch {
        console.warn(`新版已发布；旧文件可能正在使用，已保留在：${backup}`)
      }
    }
    console.log(`最新发布已输出到：${releaseDir}`)
    for (const file of files) console.log(`  ${file.name}`)
    return { releaseDir, files }
  } catch (error) {
    if (previousMoved && !published && !fs.existsSync(releaseDir)) fs.renameSync(backup, releaseDir)
    throw error
  } finally {
    if (fs.existsSync(stage) && path.dirname(stage) === root && path.basename(stage).startsWith('.release-stage-')) {
      fs.rmSync(stage, { recursive: true, force: true })
    }
  }
}

module.exports = { collectWindowsRelease }
