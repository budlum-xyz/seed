import React from 'react';
import EncodePane from './components/EncodePane.jsx';
import DecodePane from './components/DecodePane.jsx';

export default function App() {
  return (
    <div className="app">
      <header>
        <h1>Seed Transfer Interface</h1>
        <p>
          One side loads content and produces a recipe. The other side takes the
          recipe and rebuilds the content byte for byte. The digest must open, or
          the rebuild is refused.
        </p>
      </header>
      <main>
        <EncodePane />
        <DecodePane />
      </main>
    </div>
  );
}
