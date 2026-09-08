import React, { useRef, useState } from 'react';
import { encodeRecipe } from '../lib/recipe.js';

export default function EncodePane() {
  const [recipeText, setRecipeText] = useState('');
  const [meta, setMeta] = useState('');
  const [message, setMessage] = useState(null);
  const inputRef = useRef(null);

  async function handleFile(file) {
    if (!file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    try {
      const { text, digest, chunks } = await encodeRecipe(bytes, file.name);
      setRecipeText(text);
      setMeta(`${file.name} | ${bytes.length} bytes | ${chunks} chunk(s) | digest ${digest.slice(0, 16)}...`);
      setMessage({ kind: 'ok', text: `Recipe produced. The content was split into ${chunks} chunk(s); whoever holds the recipe rebuilds the content after verifying the digest.` });
    } catch (err) {
      setRecipeText('');
      setMeta('');
      setMessage({ kind: 'err', text: err.message });
    }
  }

  function downloadRecipe() {
    const blob = new Blob([recipeText], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'recipe.txt';
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <section>
      <h2>1. Content to recipe</h2>
      <div
        className="drop"
        onClick={() => inputRef.current.click()}
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => {
          e.preventDefault();
          handleFile(e.dataTransfer.files[0]);
        }}
      >
        Click to choose a file, or drop it here
      </div>
      <input
        ref={inputRef}
        type="file"
        hidden
        onChange={(e) => handleFile(e.target.files[0])}
      />
      {meta && <div className="meta">{meta}</div>}
      <textarea readOnly value={recipeText} placeholder="The recipe appears here. Copy it all, or download it." />
      <button type="button" onClick={downloadRecipe} disabled={!recipeText}>
        Download recipe
      </button>
      {message && <div className={`result ${message.kind}`}>{message.text}</div>}
    </section>
  );
}
