const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const crypto = require('crypto');

const iconsDir = __dirname;

// CRC32 表
const crcTable = (() => {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let j = 0; j < 8; j++) {
      c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    }
    table[i] = c >>> 0;
  }
  return table;
})();

function crc32(data) {
  let crc = 0xffffffff;
  for (let i = 0; i < data.length; i++) {
    crc = crcTable[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

// 创建 PNG chunk
function createChunk(type, data) {
  const typeBuffer = Buffer.from(type);
  const lengthBuffer = Buffer.alloc(4);
  lengthBuffer.writeUInt32BE(data.length, 0);
  const crcData = Buffer.concat([typeBuffer, data]);
  const crcBuffer = Buffer.alloc(4);
  crcBuffer.writeUInt32BE(crc32(crcData), 0);
  return Buffer.concat([lengthBuffer, typeBuffer, data, crcBuffer]);
}

// 创建简单的蓝色 PNG
function createBluePNG(size) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  
  // IHDR: 256x256, 8-bit, RGB (color type 2)
  const ihdrData = Buffer.alloc(13);
  ihdrData.writeUInt32BE(size, 0);  // width
  ihdrData.writeUInt32BE(size, 4);  // height
  ihdrData[8] = 8;   // bit depth
  ihdrData[9] = 2;   // color type (2 = RGB)
  ihdrData[10] = 0;  // compression
  ihdrData[11] = 0;  // filter
  ihdrData[12] = 0;  // interlace
  
  const ihdrChunk = createChunk('IHDR', ihdrData);
  
  // 创建图像数据（蓝色）
  const rowSize = 1 + size * 3; // filter byte + RGB
  const rawData = Buffer.alloc(rowSize * size);
  for (let y = 0; y < size; y++) {
    rawData[y * rowSize] = 0; // filter: none
    for (let x = 0; x < size; x++) {
      const offset = y * rowSize + 1 + x * 3;
      rawData[offset] = 0;    // R
      rawData[offset + 1] = 120;  // G
      rawData[offset + 2] = 215;  // B
    }
  }
  
  // 压缩
  const compressed = zlib.deflateSync(rawData, { level: 9 });
  const idatChunk = createChunk('IDAT', compressed);
  
  // IEND
  const iendChunk = createChunk('IEND', Buffer.alloc(0));
  
  return Buffer.concat([signature, ihdrChunk, idatChunk, iendChunk]);
}

// 生成 PNG 图标
const sizes = [32, 128, 256];
sizes.forEach(size => {
  const png = createBluePNG(size);
  const filename = path.join(iconsDir, `${size}x${size}.png`);
  fs.writeFileSync(filename, png);
  console.log(`Created: ${size}x${size}.png`);
});

// 128x128@2x
fs.copyFileSync(
  path.join(iconsDir, '256x256.png'),
  path.join(iconsDir, '128x128@2x.png')
);
console.log('Created: 128x128@2x.png');

// 生成 ICO（使用 PNG 压缩格式）
const png256 = fs.readFileSync(path.join(iconsDir, '256x256.png'));

// ICO header
const header = Buffer.alloc(6);
header.writeUInt16LE(0, 0);  // reserved
header.writeUInt16LE(1, 2);  // type (1 = icon)
header.writeUInt16LE(1, 4);  // count

// Directory entry
const dirEntry = Buffer.alloc(16);
dirEntry.writeUInt8(0, 0);    // width (0 = 256)
dirEntry.writeUInt8(0, 1);    // height (0 = 256)
dirEntry.writeUInt8(0, 2);    // color count
dirEntry.writeUInt8(0, 3);    // reserved
dirEntry.writeUInt16LE(1, 4); // planes
dirEntry.writeUInt16LE(32, 6);// bit count
dirEntry.writeUInt32LE(png256.length, 8);  // size
dirEntry.writeUInt32LE(22, 12);            // offset

const icoData = Buffer.concat([header, dirEntry, png256]);
fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoData);
console.log('Created: icon.ico');

console.log('All icons generated!');
