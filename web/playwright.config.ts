import { defineConfig } from '@playwright/test';

/** Step-1 gate: styled shell, demo mode, screenshots. Builds first. */
export default defineConfig({
  testDir: './e2e',
  timeout: 60000,
  use: {
    channel: 'chrome',
    launchOptions: {
      // Host run only: sandboxes block the sockets browsers need.
      args: ['--no-sandbox', '--disable-dev-shm-usage']
    }
  },
  webServer: {
    command: 'npm run build && npm run preview -- --port 4174 --strictPort',
    port: 4174,
    reuseExistingServer: false,
    timeout: 600000
  }
});
