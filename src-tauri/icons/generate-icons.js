const { copyFileSync, rmSync } = require('fs');
const { join, resolve } = require('path');
const { spawnSync } = require('child_process');

const iconsDir = __dirname;
const projectRoot = resolve(iconsDir, '..', '..');
const sourceSvg = join(iconsDir, 'orca.svg');
const relativeSourceSvg = 'src-tauri/icons/orca.svg';
const relativeIconsDir = 'src-tauri/icons';

const command = 'npm';
const args = ['run', 'tauri', 'icon', '--', relativeSourceSvg, '--output', relativeIconsDir];
const commandLine = `${command} run tauri icon -- ${relativeSourceSvg} --output ${relativeIconsDir}`;
const result = process.platform === 'win32'
  ? spawnSync(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', commandLine], {
      cwd: projectRoot,
      stdio: 'inherit',
    })
  : spawnSync(command, args, {
      cwd: projectRoot,
      stdio: 'inherit',
    });

if (result.status !== 0) {
  if (result.error) {
    console.error(result.error);
  }
  process.exit(result.status ?? 1);
}

// The Tauri icon generator writes 128x128@2x.png but this project config
// also references 256x256.png directly.
copyFileSync(join(iconsDir, '128x128@2x.png'), join(iconsDir, '256x256.png'));
copyFileSync(join(iconsDir, 'icon.png'), join(iconsDir, 'source.png'));

[
  '64x64.png',
  'icon.png',
  'StoreLogo.png',
  'Square30x30Logo.png',
  'Square44x44Logo.png',
  'Square71x71Logo.png',
  'Square89x89Logo.png',
  'Square107x107Logo.png',
  'Square142x142Logo.png',
  'Square150x150Logo.png',
  'Square284x284Logo.png',
  'Square310x310Logo.png',
  'android',
  'ios',
].forEach((entry) => {
  rmSync(join(iconsDir, entry), { recursive: true, force: true });
});

console.log('Synced 256x256.png and source.png');
