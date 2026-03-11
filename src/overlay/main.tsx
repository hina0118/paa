import React from 'react';
import ReactDOM from 'react-dom/client';
import { ScreenOverlay } from './ScreenOverlay';
import './overlay.css';

ReactDOM.createRoot(
  document.getElementById('overlay-root') as HTMLElement
).render(
  <React.StrictMode>
    <ScreenOverlay />
  </React.StrictMode>
);
