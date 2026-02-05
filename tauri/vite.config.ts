import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
        protocol: "ws",
        host,
        port: 1421,
      }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  // 构建优化配置
  build: {
    // 提高 chunk 大小警告限制
    chunkSizeWarningLimit: 600,

    rollupOptions: {
      output: {
        // 手动配置代码分割
        manualChunks: (id) => {
          // 将 node_modules 中的大型库分离到单独的 chunk
          if (id.includes('node_modules')) {
            // ECharts 单独打包
            if (id.includes('echarts') || id.includes('zrender')) {
              return 'echarts-vendor';
            }
            // Bootstrap 单独打包
            if (id.includes('bootstrap') || id.includes('@popperjs')) {
              return 'bootstrap-vendor';
            }
            // date-fns 单独打包
            if (id.includes('date-fns')) {
              return 'date-fns-vendor';
            }
            // Vue 相关库
            if (id.includes('vue') || id.includes('@vue')) {
              return 'vue-vendor';
            }
            // 其他 node_modules 依赖
            return 'vendor';
          }
        },
      },
    },

    // 启用 CSS 代码分割
    cssCodeSplit: true,
  },
}));
