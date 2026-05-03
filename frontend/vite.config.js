import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react-swc';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],

  build: {
    // Output to the repo root's dist/ directory (served by the Rust binary)
    outDir: '../dist',
    emptyOutDir: true,
  },

  server: {
    port: 9191,
    proxy: {
      '/api': {
        target: `http://${process.env.VITE_API_HOST || 'localhost'}:8080`,
        changeOrigin: true,
        secure: false,
      },
      '/ws': {
        target: `ws://${process.env.VITE_API_HOST || 'localhost'}:8080`,
        changeOrigin: true,
        ws: true,
      },
    },
  },

  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setupTests.js'],
    globals: true,
  },
});
