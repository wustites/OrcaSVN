const fs = require('fs');
const path = require('path');

const iconsDir = __dirname;

// 创建一个简单的蓝色方块 PNG（使用预编码的最小 PNG 数据）
function createSimplePNG(size) {
  // 这是一个最小的蓝色 PNG 模板
  const blueColor = [0, 120, 215]; // SVN 蓝色
  
  // PNG 文件结构
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  
  // IHDR chunk
  const ihdrData = Buffer.alloc(13);
  ihdrData.writeUInt32BE(size, 0);  // width
  ihdrData.writeUInt32BE(size, 4);  // height
  ihdrData[8] = 8;  // bit depth
  ihdrData[9] = 2;  // color type (RGB)
  ihdrData[10] = 0; // compression
  ihdrData[11] = 0; // filter
  ihdrData[12] = 0; // interlace
  
  const ihdrHeader = Buffer.from('IHDR');
  const ihdrCrc = crc32(Buffer.concat([ihdrHeader, ihdrData]));
  const ihdrChunk = Buffer.concat([
    Buffer.alloc(4).writeUInt32BE(13, 0) ? Buffer.alloc(4).writeUInt32BE(13, 0) && Buffer.alloc(4) : Buffer.alloc(4),
    ihdrHeader,
    ihdrData,
    Buffer.alloc(4).writeUInt32BE(ihdrCrc, 0) ? Buffer.alloc(4) : Buffer.alloc(4)
  ]);
  ihdrChunk.writeUInt32BE(13, 0);
  ihdrChunk.writeUInt32BE(ihdrCrc, 17);
  
  // IDAT chunk - 简化的压缩图像数据
  const rowSize = 1 + size * 3; // filter byte + RGB pixels
  const rawData = Buffer.alloc(rowSize * size);
  for (let y = 0; y < size; y++) {
    rawData[y * rowSize] = 0; // filter type none
    for (let x = 0; x < size; x++) {
      const offset = y * rowSize + 1 + x * 3;
      rawData[offset] = blueColor[0];
      rawData[offset + 1] = blueColor[1];
      rawData[offset + 2] = blueColor[2];
    }
  }
  
  // 使用 zlib 压缩
  const zlib = require('zlib');
  const compressed = zlib.deflateSync(rawData, { level: 9 });
  
  const idatCrc = crc32(Buffer.concat([Buffer.from('IDAT'), compressed]));
  const idatChunk = Buffer.concat([
    Buffer.alloc(4).writeUInt32BE(compressed.length, 0) ? Buffer.alloc(4) : Buffer.alloc(4),
    Buffer.from('IDAT'),
    compressed,
    Buffer.alloc(4)
  ]);
  idatChunk.writeUInt32BE(compressed.length, 0);
  idatChunk.writeUInt32BE(idatCrc, idatChunk.length - 4);
  
  // IEND chunk
  const iendCrc = crc32(Buffer.from('IEND'));
  const iendChunk = Buffer.concat([
    Buffer.alloc(4).writeUInt32BE(0, 0) ? Buffer.alloc(4) : Buffer.alloc(4),
    Buffer.from('IEND'),
    Buffer.alloc(4)
  ]);
  iendChunk.writeUInt32BE(iendCrc, 4);
  
  return Buffer.concat([signature, ihdrChunk, idatChunk, iendChunk]);
}

function crc32(data) {
  const table = [];
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let j = 0; j < 8; j++) {
      c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    }
    table[i] = c >>> 0;
  }
  
  let crc = 0xffffffff;
  for (let i = 0; i < data.length; i++) {
    crc = table[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

// 生成图标
const sizes = [32, 128, 256];
sizes.forEach(size => {
  const png = createSimplePNG(size);
  const filename = path.join(iconsDir, `${size}x${size}.png`);
  fs.writeFileSync(filename, png);
  console.log(`Created: ${size}x${size}.png`);
});

// 128x128@2x 就是 256x256
fs.copyFileSync(
  path.join(iconsDir, '256x256.png'),
  path.join(iconsDir, '128x128@2x.png')
);
console.log('Created: 128x128@2x.png');

console.log('All icons generated successfully!');
