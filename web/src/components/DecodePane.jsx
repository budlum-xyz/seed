import React, { useState } from 'react';
import { decodeRecipe } from '../lib/recipe.js';

export default function DecodePane() {
  const [text, setText] = useState('');
  const [message, setMessage] = useState(null);

  async function rebuild() {
    try {
      const { name, bytes } = await decodeRecipe(text);
      const blob = new Blob([bytes], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.click();
      URL.revokeObjectURL(url);
      setMessage({ kind: 'ok', text: `Rebuild complete: ${bytes.length} bytes restored, digest opened byte for byte, file downloaded.` });
    } catch (err) {
      setMessage({ kind: 'err', text: err.message });
    }
  }

  return (
    <section>
      <h2>2. Recipe to content</h2>
      <textarea value={text} onChange={(e) => setText(e.target.value)} placeholder="Paste the recipe text here." />
      <button type="button" onClick={rebuild}>
        Rebuild content
      </button>
      {message && <div className={`result ${message.kind}`}>{message.text}</div>}
    </section>
  );
}
