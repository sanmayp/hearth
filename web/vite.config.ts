import { sveltekit } from '@sveltejs/kit/vite';
import { SvelteKitPWA } from '@vite-pwa/sveltekit';
import { defineConfig, loadEnv } from 'vite';

/** PWA plugin owns the service worker. The app never reloads mid-task. */
export default defineConfig(({ mode }) => {
  // Forward API calls to the server. Shell wins, then .env file, then 8080.
  const fileEnv = loadEnv(mode, '.');
  const procEnv =
    (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env ?? {};
  const api = procEnv.HEARTH_API ?? fileEnv.HEARTH_API ?? 'http://127.0.0.1:8080';
  return {
    // Expose PUBLIC_* to the client (API base). Else demo-only.
    envPrefix: ['VITE_', 'PUBLIC_'],
    plugins: [
      sveltekit(),
      SvelteKitPWA({
        registerType: 'prompt',
        manifest: false,
        workbox: { globPatterns: ['client/**/*.{js,css,svg,png,ico,woff2,webp,mp4}'] }
      })
    ],
    server: {
      proxy: {
        '/v1': api
      }
    },
    preview: {
      cors: false,
      allowedHosts: ['example.ts.net'],
      proxy: {
        '/v1': api
      }
    }
    // Note: vitest default include covers src/**/*.test.ts. No test key here.
    // A test key needs vitest/config types, which clash with the root Vite copy.
  };
});
