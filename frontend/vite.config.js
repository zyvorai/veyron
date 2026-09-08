import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Phase 1 of the veyron React migration: this app is served additively at
// /console alongside the existing vanilla-JS dashboard at /dashboard (see
// src/api/http_server.rs). base must match the route it's served from so
// Vite emits correctly-prefixed asset URLs.
export default defineConfig({
  plugins: [react()],
  base: '/console/',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/api': {
        target: process.env.VEYRON_API_PROXY || 'https://127.0.0.1:5151',
        changeOrigin: true,
        secure: false,
      },
    },
  },
});
