import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// 前端纯逻辑单测: 不经 SvelteKit 插件(被测模块无 DOM/Tauri 运行时依赖),
// node 环境直跑 src 下的 *.test.ts; $lib 别名与 svelte.config 保持一致
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
  resolve: {
    alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) },
  },
});
