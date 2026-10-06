import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // target/ лочит файлы на Windows — watcher туда лезть не должен
    watch: {
      ignored: ['**/src-tauri/target/**'],
    },
  },
  build: {
    target: 'esnext',
  },
})
