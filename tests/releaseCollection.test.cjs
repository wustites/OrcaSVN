const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const crypto = require('node:crypto')
const { resolveReleaseConfig } = require('../scripts/release-config.cjs')
const { collectWindowsRelease } = require('../scripts/collect-release.cjs')

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'orcasvn-release-test-'))
  t.after(() => fs.rmSync(root, { recursive: true, force: true }))
  fs.mkdirSync(path.join(root, 'src-tauri'))
  fs.writeFileSync(path.join(root, 'src-tauri/tauri.conf.json'), JSON.stringify({ productName: 'OrcaSVN', version: '1.0.0', bundle: { active: true, targets: 'all' } }))
  fs.writeFileSync(path.join(root, 'LICENSE'), 'license')
  const outputDir = path.join(root, 'output')
  fs.mkdirSync(outputDir)
  fs.writeFileSync(path.join(outputDir, 'OrcaSVN.exe'), 'binary')
  return { root, outputDir, version: '1.0.0', startedAt: Date.now() }
}

test('release config applies platform, file and repeated inline patches in order', t => {
  const { root } = fixture(t)
  fs.writeFileSync(path.join(root, 'src-tauri/tauri.windows.conf.json'), '{"version":"2.0.0","bundle":{"targets":["nsis"]}}')
  fs.writeFileSync(path.join(root, 'override.json'), '{"version":"3.0.0"}')
  assert.deepEqual(resolveReleaseConfig(root, ['build'], {}), { version: '2.0.0', expectedBundles: ['nsis'] })
  assert.deepEqual(resolveReleaseConfig(root, ['build', '-c', 'override.json', '--config={"version":"9.9.9"}', '-b', 'msi'], {}), { version: '9.9.9', expectedBundles: ['msi'] })
  assert.deepEqual(resolveReleaseConfig(root, ['build', '--no-bundle'], {}).expectedBundles, [])
  assert.deepEqual(resolveReleaseConfig(root, ['build', '-c', '{"bundle":{"active":false}}'], {}).expectedBundles, [])
  assert.deepEqual(resolveReleaseConfig(root, ['build', '--bundles=nsis,msi'], {}).expectedBundles, ['nsis', 'msi'])
  assert.throws(() => resolveReleaseConfig(root, ['build', '-c', 'override.toml'], {}), /JSON/)
  assert.throws(() => resolveReleaseConfig(root, ['build'], { TAURI_CONFIG: '{}' }), /TAURI_CONFIG/)
})

test('missing, mismatched or stale installers preserve the previous release', t => {
  const options = fixture(t)
  const release = path.join(options.root, 'release')
  fs.mkdirSync(release)
  fs.writeFileSync(path.join(release, 'keep.txt'), 'previous')
  const bundle = path.join(options.outputDir, 'bundle/nsis')
  assert.throws(() => collectWindowsRelease({ ...options, expectedBundles: ['nsis'] }), /nsis/)
  fs.mkdirSync(bundle, { recursive: true })
  fs.writeFileSync(path.join(bundle, 'OrcaSVN_0.9.0_x64-setup.exe'), 'wrong version')
  assert.throws(() => collectWindowsRelease({ ...options, expectedBundles: ['nsis'] }), /nsis/)
  const installer = path.join(bundle, 'OrcaSVN_1.0.0_x64-setup.exe')
  fs.writeFileSync(installer, 'stale')
  fs.utimesSync(installer, new Date(0), new Date(0))
  assert.throws(() => collectWindowsRelease({ ...options, expectedBundles: ['nsis'] }), /nsis/)
  assert.equal(fs.readFileSync(path.join(release, 'keep.txt'), 'utf8'), 'previous')
})

for (const expectedBundles of [[], ['nsis'], ['msi'], ['nsis', 'msi']]) {
  test(`native collection validates manifest and preserves extras: ${expectedBundles}`, { skip: process.platform !== 'win32' }, t => {
    const options = fixture(t)
    for (const kind of expectedBundles) {
      const dir = path.join(options.outputDir, 'bundle', kind)
      fs.mkdirSync(dir, { recursive: true })
      fs.writeFileSync(path.join(dir, `OrcaSVN_1.0.0_x64.${kind === 'nsis' ? 'exe' : 'msi'}`), kind)
    }
    const { releaseDir } = collectWindowsRelease({ ...options, expectedBundles })
    fs.writeFileSync(path.join(releaseDir, 'user.txt'), 'user data')
    const { files } = collectWindowsRelease({ ...options, expectedBundles })
    assert.equal(fs.readFileSync(path.join(releaseDir, 'user.txt'), 'utf8'), 'user data')
    for (const file of files) {
      const bytes = fs.readFileSync(path.join(releaseDir, file.name))
      assert.equal(file.bytes, bytes.length)
      assert.equal(file.sha256, crypto.createHash('sha256').update(bytes).digest('hex'))
    }
    fs.writeFileSync(path.join(releaseDir, 'release-manifest.json'), JSON.stringify({ generatedBy: 'OrcaSVN/collect-release', files: [] }))
    assert.throws(() => collectWindowsRelease({ ...options, expectedBundles }), /非托管/)
    assert.equal(fs.readFileSync(path.join(releaseDir, 'user.txt'), 'utf8'), 'user data')
  })
}

test('publication rename failure restores the previous release', { skip: process.platform !== 'win32' }, t => {
  const options = fixture(t)
  const { releaseDir } = collectWindowsRelease({ ...options, expectedBundles: [] })
  const previous = fs.readFileSync(path.join(releaseDir, 'release-manifest.json'))
  fs.writeFileSync(path.join(releaseDir, 'user.txt'), 'preserve me')
  const rename = fs.renameSync
  t.mock.method(fs, 'renameSync', (source, target) => {
    if (path.basename(source).startsWith('.release-stage-') && target === releaseDir) {
      throw new Error('simulated publication failure')
    }
    return rename(source, target)
  })
  assert.throws(() => collectWindowsRelease({ ...options, expectedBundles: [] }), /simulated publication failure/)
  assert.deepEqual(fs.readFileSync(path.join(releaseDir, 'release-manifest.json')), previous)
  assert.equal(fs.readFileSync(path.join(releaseDir, 'user.txt'), 'utf8'), 'preserve me')
  assert.deepEqual(fs.readdirSync(options.root).filter(name => name.startsWith('.release-')), [])
})
