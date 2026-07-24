/**
 * Genera el PNG fuente del icono (1024x1024) sin dependencias externas.
 * Se ejecuta una sola vez; luego `pnpm tauri icon` deriva el resto de tamanos.
 *
 * Uso: node scripts/generate-icon.mjs
 */
import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SIZE = 1024;
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(root, 'src-tauri', 'icons', 'source.png');

/** Buffer RGBA. */
const px = Buffer.alloc(SIZE * SIZE * 4, 0);

function setPixel(x, y, [r, g, b, a]) {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return;
  const i = (y * SIZE + x) * 4;
  const srcA = a / 255;
  const dstA = px[i + 3] / 255;
  const outA = srcA + dstA * (1 - srcA);
  if (outA === 0) return;
  px[i] = Math.round((r * srcA + px[i] * dstA * (1 - srcA)) / outA);
  px[i + 1] = Math.round((g * srcA + px[i + 1] * dstA * (1 - srcA)) / outA);
  px[i + 2] = Math.round((b * srcA + px[i + 2] * dstA * (1 - srcA)) / outA);
  px[i + 3] = Math.round(outA * 255);
}

/** Rectangulo redondeado con antialiasing por distancia al borde. */
function roundedRect(x0, y0, w, h, radius, color) {
  const cx0 = x0 + radius;
  const cy0 = y0 + radius;
  const cx1 = x0 + w - radius;
  const cy1 = y0 + h - radius;
  for (let y = Math.floor(y0) - 2; y < Math.ceil(y0 + h) + 2; y++) {
    for (let x = Math.floor(x0) - 2; x < Math.ceil(x0 + w) + 2; x++) {
      const qx = Math.min(Math.max(x + 0.5, cx0), cx1);
      const qy = Math.min(Math.max(y + 0.5, cy0), cy1);
      const dist = Math.hypot(x + 0.5 - qx, y + 0.5 - qy);
      const coverage = Math.min(Math.max(radius + 0.5 - dist, 0), 1);
      if (coverage <= 0) continue;
      setPixel(x, y, [color[0], color[1], color[2], Math.round(color[3] * coverage)]);
    }
  }
}

// Fondo: cuadrado redondeado grafito.
roundedRect(0, 0, SIZE, SIZE, 224, [24, 24, 27, 255]);

// Rejilla 3x3 de slots.
const pad = 208;
const gap = 44;
const cell = (SIZE - pad * 2 - gap * 2) / 3;
const accent = [125, 211, 252, 255]; // sky-300
const muted = [82, 82, 91, 255]; // zinc-600
const active = [56, 189, 248, 255]; // sky-400

for (let row = 0; row < 3; row++) {
  for (let col = 0; col < 3; col++) {
    const x = pad + col * (cell + gap);
    const y = pad + row * (cell + gap);
    const isActive = row === 1 && col === 1;
    roundedRect(x, y, cell, cell, cell * 0.28, isActive ? active : muted);
    if (isActive) {
      // Punto de "reproduciendo" en el centro.
      roundedRect(
        x + cell * 0.34,
        y + cell * 0.34,
        cell * 0.32,
        cell * 0.32,
        cell * 0.16,
        [24, 24, 27, 255],
      );
    }
  }
}

// Barra superior tipo "onda".
const barBase = 128;
const bars = [0.45, 0.85, 0.6, 1, 0.7];
const barW = 34;
const barGap = 22;
const totalW = bars.length * barW + (bars.length - 1) * barGap;
let bx = (SIZE - totalW) / 2;
for (const factor of bars) {
  const h = barBase * factor;
  roundedRect(bx, pad - 40 - h, barW, h, barW / 2, accent);
  bx += barW + barGap;
}

/** Codifica el buffer RGBA como PNG. */
function encodePng(width, height, rgba) {
  const raw = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0; // filtro None
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }

  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buf) {
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return c ^ -1;
}

mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, encodePng(SIZE, SIZE, px));
console.warn(`Icono generado: ${out}`);
