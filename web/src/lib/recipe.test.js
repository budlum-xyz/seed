import { describe, expect, it } from 'vitest';
import { base32Decode, base32Encode, decodeRecipe, encodeRecipe, sha256Hex } from './recipe.js';

// Node >= 20 exposes WebCrypto as globalThis.crypto.
function bytes(n) {
  const out = new Uint8Array(n);
  for (let i = 0; i < n; i += 1) out[i] = (i * 31 + 7) & 255;
  return out;
}

describe('base32', () => {
  it('round-trips every boundary length', () => {
    for (const n of [1, 4, 239, 240, 241, 1000, 4096]) {
      const src = bytes(n);
      expect(base32Decode(base32Encode(src))).toEqual(src);
    }
  });
});

describe('recipe transfer', () => {
  it('rebuilds the content byte for byte', async () => {
    const content = bytes(5000);
    const { text, digest } = await encodeRecipe(content, 'sample.bin');
    const back = await decodeRecipe(text);
    expect(back.name).toBe('sample.bin');
    expect(back.bytes).toEqual(content);
    expect(back.digest).toBe(digest);
    expect(await sha256Hex(back.bytes)).toBe(digest);
  });

  it('refuses a tampered chunk', async () => {
    const content = bytes(1000);
    const { text } = await encodeRecipe(content, 'sample.bin');
    const tampered = text.replace(/CHUNK 2 [A-Z2-7]+/, (m) => `${m.slice(0, -4)}AAAA`);
    await expect(decodeRecipe(tampered)).rejects.toThrow(/digest mismatch/);
  });

  it('refuses a missing chunk', async () => {
    const content = bytes(1000);
    const { text } = await encodeRecipe(content, 'sample.bin');
    const lines = text.split('\n');
    const cut = lines.filter((l) => !l.startsWith('CHUNK 3 ')).join('\n');
    await expect(decodeRecipe(cut)).rejects.toThrow(/missing chunks/);
  });

  it('refuses an unknown format', async () => {
    await expect(decodeRecipe('NOT-A-RECIPE\n')).rejects.toThrow(/unknown recipe format/);
  });

  it('refuses empty content at encode time', async () => {
    await expect(encodeRecipe(new Uint8Array(0), 'empty.bin')).rejects.toThrow(/empty content/);
  });
});
