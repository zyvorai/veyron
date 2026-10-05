import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  base: '/console/',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined;
          if (id.includes('@novnc')) return 'novnc';
          if (id.includes('lucide-react')) return 'icons';
          if (/[\\/](react|react-dom|scheduler)[\\/]/.test(id)) return 'react';
          return undefined;
        },
      },
    },
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
