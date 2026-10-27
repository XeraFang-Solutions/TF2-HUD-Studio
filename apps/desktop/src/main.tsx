import React from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource/barlow-semi-condensed/500.css';
import '@fontsource/barlow-semi-condensed/600.css';
import '@fontsource/barlow-semi-condensed/700.css';
import './style.css';
import { App } from './App.tsx';
import { applyCachedTheme } from './appearance.ts';

applyCachedTheme();

createRoot(document.getElementById('root')!).render(<App />);
