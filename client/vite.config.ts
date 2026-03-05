import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
    plugins: [wasm(), topLevelAwait()],
    publicDir: '../Assets',
    build: {
        target: 'es2022',
        outDir: 'dist',
        assetsInlineLimit: 0,
    },
    optimizeDeps: {
        exclude: ['machine-empire-wasm'],
    },
    server: {
        headers: {
            'Cross-Origin-Opener-Policy': 'same-origin',
            'Cross-Origin-Embedder-Policy': 'require-corp',
        },
    },
});
