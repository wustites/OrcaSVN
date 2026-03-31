const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const iconsDir = __dirname;

// 读取现有的 PNG 文件并转换为 ICO
function pngToIco(pngPath) {
  const pngData = fs.readFileSync(pngPath);
  
  // ICO 文件头：ICONDIR 结构 (6 字节)
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);  // idReserved
  header.writeUInt16LE(1, 2);  // idType (1 = icon)
  header.writeUInt16LE(1, 4);  // idCount (1 image)
  
  // ICONDIRENTRY 结构 (16 字节)
  const dirEntry = Buffer.alloc(16);
  dirEntry.writeUInt8(0, 0);    // bWidth (0 = 256)
  dirEntry.writeUInt8(0, 1);    // bHeight (0 = 256)
  dirEntry.writeUInt8(0, 2);    // bColorCount
  dirEntry.writeUInt8(0, 3);    // bReserved
  dirEntry.writeUInt16LE(1, 4); // wPlanes
  dirEntry.writeUInt16LE(32, 6);// wBitCount (32-bit)
  
  // PNG 数据大小
  const pngSize = pngData.length;
  dirEntry.writeUInt32LE(pngSize, 8);  // dwBytesInRes
  
  // 数据偏移 (6 + 16 = 22)
  dirEntry.writeUInt32LE(22, 12);      // dwImageOffset
  
  // 组合 ICO 文件
  return Buffer.concat([header, dirEntry, pngData]);
}

// 生成 ICO 文件
const pngPath = path.join(iconsDir, '256x256.png');
const icoData = pngToIco(pngPath);
fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoData);
console.log('Created: icon.ico (256x256 32-bit PNG)');
console.log('Done!');
