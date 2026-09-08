// Seed recipe format: the text artifact that travels instead of the content.
//
//   SEED-RECIPE-1
//   NAME <original file name>
//   SIZE <byte length>
//   DIGEST <sha256 hex of the original bytes>
//   CHUNKS <chunk count>
//   CHUNK <1-based index> <base32 chunk>
//   END
//
// The reverse direction verifies before it returns: a rebuild only succeeds
// when the size and the digest both open.

export const FORMAT = 'SEED-RECIPE-1';
export const CHUNK_BYTES = 240;
const B32 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

export function base32Encode(bytes) {
  let out = '';
  let bits = 0;
  let value = 0;
  for (let i = 0; i < bytes.length; i += 1) {
    value = (value << 8) | bytes[i];
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) {
    out += B32[(value << (5 - bits)) & 31];
  }
  return out;
}

export function base32Decode(text) {
  let bits = 0;
  let value = 0;
  const out = [];
  for (const ch of text) {
    const d = B32.indexOf(ch.toUpperCase());
    if (d < 0) continue;
    value = (value << 5) | d;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 255);
      bits -= 8;
    }
  }
  return Uint8Array.from(out);
}

export async function sha256Hex(bytes) {
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

export async function encodeRecipe(bytes, name) {
  if (bytes.length === 0) {
    throw new Error('empty content is refused: a zero-length transfer is not a valid unit');
  }
  const digest = await sha256Hex(bytes);
  const count = Math.max(1, Math.ceil(bytes.length / CHUNK_BYTES));
  const lines = [FORMAT, `NAME ${name}`, `SIZE ${bytes.length}`, `DIGEST ${digest}`, `CHUNKS ${count}`];
  for (let i = 0; i < count; i += 1) {
    const chunk = bytes.subarray(i * CHUNK_BYTES, (i + 1) * CHUNK_BYTES);
    lines.push(`CHUNK ${i + 1} ${base32Encode(chunk)}`);
  }
  lines.push('END');
  return { text: lines.join('\n'), digest, chunks: count };
}

export async function decodeRecipe(text) {
  const lines = text.split(/\r?\n/);
  if (lines[0] !== FORMAT) {
    throw new Error(`unknown recipe format: the first line must be ${FORMAT}`);
  }
  let name = 'content';
  let size = -1;
  let digest = '';
  let expected = -1;
  const chunks = [];
  for (const line of lines) {
    if (line.startsWith('NAME ')) name = line.slice(5).trim();
    else if (line.startsWith('SIZE ')) size = parseInt(line.slice(5), 10);
    else if (line.startsWith('DIGEST ')) digest = line.slice(7).trim();
    else if (line.startsWith('CHUNKS ')) expected = parseInt(line.slice(7), 10);
    else if (line.startsWith('CHUNK ')) {
      const gap = line.indexOf(' ', 6);
      if (gap < 0) throw new Error('malformed chunk line');
      chunks.push({ index: parseInt(line.slice(6, gap), 10), data: base32Decode(line.slice(gap + 1)) });
    }
  }
  if (size < 0 || !digest || expected < 1) {
    throw new Error('incomplete recipe: SIZE, DIGEST and CHUNKS are all required');
  }
  if (chunks.length !== expected) {
    throw new Error(`missing chunks: ${chunks.length}/${expected} present, rebuild refused`);
  }
  chunks.sort((a, b) => a.index - b.index);
  let total = 0;
  chunks.forEach((c) => {
    total += c.data.length;
  });
  const joined = new Uint8Array(total);
  let offset = 0;
  chunks.forEach((c) => {
    joined.set(c.data, offset);
    offset += c.data.length;
  });
  const found = await sha256Hex(joined);
  if (found !== digest) {
    throw new Error(`digest mismatch: expected ${digest.slice(0, 16)}..., found ${found.slice(0, 16)}...; output refused`);
  }
  return { name, bytes: joined, digest };
}
