import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ mode }) => {
  // 加载上级目录的 .env 文件
  const env = loadEnv(mode, '../', ['VITE_'])

  return {
    plugins: [react()],
    build: {
      outDir: '../static',
      emptyOutDir: true,
    },
    define: {
      // 手动定义环境变量，确保它们在构建时可用
      'import.meta.env.VITE_BOT_USERNAME': JSON.stringify(env.VITE_BOT_USERNAME),
      'import.meta.env.VITE_REDIRECT_URL': JSON.stringify(env.VITE_REDIRECT_URL),
    }
  }
})