import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

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
