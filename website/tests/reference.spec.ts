import {readFileSync} from 'node:fs';
import {join} from 'node:path';
import {expect, test} from '@playwright/test';

const ROOT = join(__dirname, '..', '..');
const PAGES = [
  {
    path: '/inbox/docs/reference/http-api',
    title: 'HTTP API',
    names: ['/messaging/commands/post', '/messaging/views/pending', 'inbox.messaging.PostMessage'],
    served: '/inbox/reference/http-api.yaml',
    source: 'generated/openapi/openapi/inbox-service.yaml',
  },
  {
    path: '/inbox/docs/reference/events',
    title: 'Events',
    names: ['MessagePosted', 'MessageAnswered', 'DecisionTakenOnSilence'],
    served: '/inbox/reference/events.yaml',
    source: 'generated/asyncapi/asyncapi/inbox-service.yaml',
  },
  {
    path: '/inbox/docs/reference/cli',
    title: 'Command line',
    names: ['inbox post', 'inbox serve', 'INBOX_URL', '--max-connections'],
    served: '/inbox/reference/cli.json',
    source: 'generated/cli/inbox.json',
  },
];

for (const reference of PAGES) {
  test(`${reference.title}: the page names what its document declares`, async ({page}) => {
    await page.goto(reference.path);
    await expect(page.locator('h1')).toHaveText(reference.title);
    const article = page.locator('article');
    for (const name of reference.names) await expect(article.getByText(name, {exact: false}).first()).toBeAttached();
  });

  test(`${reference.title}: the page links its document, and the site serves the committed bytes`, async ({page, request}) => {
    await page.goto(reference.path);
    await expect(page.locator(`article a[href="${reference.served}"]`).first()).toBeVisible();
    const served = await request.get(reference.served);
    expect(served.status()).toBe(200);
    expect(await served.text()).toBe(readFileSync(join(ROOT, reference.source), 'utf8'));
  });

  test(`${reference.title}: the page is in the sidebar and fits a phone`, async ({page}) => {
    await page.goto(reference.path);
    // The category's own link points at its first page: leave it out.
    await expect(
      page.locator(`nav.menu a.menu__link:not(.menu__link--sublist)[href="${reference.path}"]`),
    ).toHaveText(reference.title);
    await page.setViewportSize({width: 390, height: 844});
    await page.goto(reference.path);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });
}

test('the HTTP API page sends no request to the service it describes', async ({page}) => {
  const outside: string[] = [];
  page.on('request', (sent) => {
    if (!sent.url().startsWith('http://127.0.0.1:') || sent.url().includes('/messaging/')) outside.push(sent.url());
  });
  await page.goto('/inbox/docs/reference/http-api');
  await expect(page.locator('article form, article input[type=password], article button[type=submit]')).toHaveCount(0);
  expect(outside).toEqual([]);
});
