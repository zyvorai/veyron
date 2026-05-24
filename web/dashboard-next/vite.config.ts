import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  base: '/dashboard-next/',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'esnext',
  },
  server: {
    proxy: {
      '/api': 'http://localhost:5151',
      '/ws': {
        target: 'ws://localhost:5151',
        ws: true,
      },
    },
  },
});
