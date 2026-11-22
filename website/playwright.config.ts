import {defineConfig, devices} from '@playwright/test';

const port = Number(process.env.SITE_PORT ?? 3421);

export default defineConfig({
  testDir: './tests',
  outputDir: '../.out/playwright',
  fullyParallel: true,
  reporter: [['list']],
  use: {
    ...devices['Desktop Chrome'],
    baseURL: `http://127.0.0.1:${port}`,
    viewport: {width: 1440, height: 900},
  },
  webServer: {
    command: `npx docusaurus serve --dir build --port ${port} --host 127.0.0.1 --no-open`,
    url: `http://127.0.0.1:${port}/inbox/`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
