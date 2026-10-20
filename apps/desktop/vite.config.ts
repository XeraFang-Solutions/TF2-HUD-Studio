import { defineConfig } from 'vite';
export default defineConfig({ server: { port: 1420, strictPort: true }, clearScreen: false, build: { chunkSizeWarningLimit: 1500 } });
