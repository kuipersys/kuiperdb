import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/db': {
        target: 'http://localhost:17001',
        changeOrigin: true,
      },
      '/logs': {
        target: 'http://localhost:17001',
        changeOrigin: true,
      },
      '/health': {
        target: 'http://localhost:17001',
        changeOrigin: true,
      }
    }
  }
})
