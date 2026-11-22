import {readFileSync} from 'node:fs';
import {join} from 'node:path';
import {expect, test, type Locator, type Page} from '@playwright/test';
import animation from '../src/generated/animation.json';
import product from '../product.json';

const HOME = '/inbox/';
const THEMES = ['light', 'dark'] as const;
const LABELS = ['What it does', 'Why it is simple', 'Why it is solid'];
const MIN_PX = 13;
const words = (text: string) => text.split(/\s+/).filter(Boolean).length;

/** Opens the landing page in a theme, chosen the way a reader's system chooses it (the stored choice's key is namespaced per site). */
async function open(page: Page, theme: 'light' | 'dark' = 'light') {
  await page.emulateMedia({colorScheme: theme});
  await page.goto(HOME);
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
}

/** Text of the hero figure a reader sees: every text element that is not hidden from view or drawn as a glyph. */
async function heroWords(page: Page): Promise<string[]> {
  return page.locator('[data-testid=inbox-hero] svg text:not([aria-hidden=true])').allTextContents();
}

const activeSlide = (page: Page) => page.locator('[data-testid=pitch-slide][aria-hidden=false]');

/** The rendered size in px of every text of a root a reader can see: SVG text scaled by its figure, HTML text as it is. */
async function textSizes(root: Locator) {
  return root.evaluate((element) => {
    const sizes: {text: string; px: number; family: string}[] = [];
    element.querySelectorAll('svg text').forEach((node) => {
      if (node.closest('[aria-hidden=true]:not(text)')) return;
      const svg = (node as SVGTextElement).ownerSVGElement!;
      const style = getComputedStyle(node);
      sizes.push({text: node.textContent ?? '', px: parseFloat(style.fontSize) * svg.getScreenCTM()!.a, family: style.fontFamily});
    });
    const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const parent = node.parentElement!;
      if (!node.textContent?.trim() || parent.closest('svg') || parent.closest('[aria-hidden=true]')) continue;
      const style = getComputedStyle(parent);
      sizes.push({text: node.textContent.trim(), px: parseFloat(style.fontSize), family: style.fontFamily});
    }
    return sizes;
  });
}

test.describe('hero figure', () => {
  test('is on the first screen at 1440 x 900, fills its column, with 3 agents, 2 hosts and the planned mark', async ({page}) => {
    await open(page);
    const hero = page.getByTestId('inbox-hero');
    await expect(hero).toHaveAttribute('data-layout', 'wide');
    const box = (await hero.boundingBox())!;
    expect(box.y).toBeGreaterThanOrEqual(0);
    expect(box.y + box.height).toBeLessThanOrEqual(900);
    const column = (await page.locator('.b10x-hero__aside').boundingBox())!;
    expect(box.width / column.width).toBeGreaterThan(0.99);
    const drawn = (await hero.locator('svg').boundingBox())!;
    expect(drawn.width / column.width).toBeGreaterThan(0.94);
    await expect(hero.locator('.inbox-hero__agent')).toHaveCount(3);
    await expect(hero.locator('.inbox-hero__host')).toHaveCount(2);
    await expect(hero.locator('.inbox-hero__process')).toHaveCount(4);
    const planned = page.getByTestId('hero-planned');
    await expect(planned.locator('text', {hasText: animation.layout.planned})).toBeVisible();
    await expect(planned.locator('.inbox-hero__agent')).toHaveCount(1);
    await expect(planned.locator('.inbox-hero__host')).toHaveCount(1);
    await expect(planned.locator('.inbox-hero__link--planned')).toHaveCount(1);
    for (const shape of await planned.locator('.inbox-hero__host rect, .inbox-hero__link--planned').all()) {
      expect(await shape.evaluate((node) => getComputedStyle(node).strokeDasharray)).not.toBe('none');
    }
  });

  test('shows the still under reduced motion: one message and its operation, no frames', async ({page}) => {
    await page.emulateMedia({reducedMotion: 'reduce'});
    await page.clock.install();
    await open(page);
    const hero = page.getByTestId('inbox-hero');
    await expect(hero).toHaveAttribute('data-mode', 'still');
    await page.clock.runFor(animation.frame_ms * 3);
    await expect(page.getByTestId('hero-frame')).toHaveCount(0);
    await expect(page.getByTestId('hero-still')).toContainText(animation.still[0].command);
    expect(words((await heroWords(page)).join(' '))).toBeLessThanOrEqual(animation.word_limit);
  });

  test('animates every frame of the generated data with at most 12 words on screen', async ({page}) => {
    await page.emulateMedia({reducedMotion: 'no-preference'});
    await page.clock.install();
    await open(page);
    await expect(page.getByTestId('inbox-hero')).toHaveAttribute('data-mode', 'animated');
    const seen: string[] = [];
    for (const frame of animation.frames) {
      const current = page.getByTestId('hero-frame');
      await expect(current).toHaveAttribute('data-command', frame.command);
      const shown = await heroWords(page);
      const count = words(shown.join(' '));
      expect(count, shown.join(' ')).toBeLessThanOrEqual(12);
      expect(count, shown.join(' ')).toBe(frame.words);
      seen.push(frame.command);
      await page.clock.runFor(animation.frame_ms);
    }
    expect(new Set(seen)).toEqual(new Set(animation.commands));
  });

  test(`labels are at least ${MIN_PX} px at 1440 and in one font family`, async ({page}) => {
    await open(page);
    const sizes = await textSizes(page.getByTestId('inbox-hero'));
    expect(sizes.length).toBeGreaterThan(4);
    for (const size of sizes) expect(size.px, size.text).toBeGreaterThanOrEqual(MIN_PX);
    expect(new Set(sizes.map((s) => s.family)).size).toBe(1);
  });
});

/** Contrast of every visible text against what is drawn under it, and of every stroked shape and outline against both of its sides. */
async function contrast(root: Locator) {
  return root.evaluate((rootElement) => {
    // Measure the state each fade and each move ends in, not a moment inside it.
    document.getAnimations().forEach((animation) => animation.finish());
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 1;
    const context = canvas.getContext('2d', {willReadFrequently: true})!;
    const rgba = (colour: string): number[] => {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = '#000';
      context.fillStyle = colour;
      context.fillRect(0, 0, 1, 1);
      const [r, g, b, a] = context.getImageData(0, 0, 1, 1).data;
      return [r, g, b, a / 255];
    };
    const probe = document.createElement('div');
    probe.style.color = 'var(--b10x-color-canvas)';
    document.body.appendChild(probe);
    const page = rgba(getComputedStyle(probe).color);
    probe.remove();
    const over = (top: number[], bottom: number[]) => [0, 1, 2].map((i) => top[i] * top[3] + bottom[i] * (1 - top[3])).concat(1);
    const luminance = (c: number[]) => {
      const [r, g, b] = c.slice(0, 3).map((v) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const ratio = (a: number[], b: number[]) => {
      const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
      return (x + 0.05) / (y + 0.05);
    };
    const opacity = (node: Element) => {
      let value = 1;
      for (let n: Element | null = node; n; n = n.parentElement) value *= parseFloat(getComputedStyle(n).opacity || '1');
      return value;
    };
    const paint = (node: Element, property: 'fill' | 'stroke' | 'color' | 'borderTopColor') => {
      const colour = rgba(getComputedStyle(node)[property]);
      colour[3] *= opacity(node);
      return colour;
    };
    /** The background of an HTML element: the first ancestor that paints one, over the page. */
    const htmlGround = (node: Element | null): number[] => {
      for (let n = node; n; n = n.parentElement) {
        const colour = rgba(getComputedStyle(n).backgroundColor);
        if (colour[3] > 0) return over(colour, page);
      }
      return page;
    };
    const FILLED = new Set(['rect', 'circle', 'ellipse', 'path', 'polygon']);
    /**
     * What is painted at a point of the screen under `shape`: the topmost filled SVG shape that is `shape` itself or is
     * painted before it. A shape painted after it (the message resting on the tray, say) covers it and is not its ground.
     */
    const groundAt = (x: number, y: number, svg: SVGSVGElement, shape: Element, self = true): number[] => {
      for (const node of document.elementsFromPoint(x, y)) {
        if (node === shape && !self) continue;
        if (node !== shape && !(node.compareDocumentPosition(shape) & Node.DOCUMENT_POSITION_FOLLOWING)) continue;
        if (!(node instanceof SVGGeometryElement) || !FILLED.has(node.tagName)) continue;
        if (!svg.contains(node)) continue;
        const fill = getComputedStyle(node).fill;
        if (!fill || fill === 'none') continue;
        const colour = paint(node, 'fill');
        if (colour[3] === 0) continue;
        return over(colour, htmlGround(svg));
      }
      return htmlGround(svg);
    };
    const visible = (node: Element) => {
      if (node.closest('[data-testid=pitch-slide][aria-hidden=true]')) return false;
      const box = node.getBoundingClientRect();
      return box.width > 0 && box.height > 0 && box.right > 0 && box.bottom > 0 && box.left < innerWidth && box.top < innerHeight && opacity(node) > 0.05;
    };
    const results: {kind: 'text' | 'line'; what: string; ratio: number}[] = [];

    rootElement.querySelectorAll('svg text').forEach((text) => {
      if (!visible(text)) return;
      const svg = (text as SVGTextElement).ownerSVGElement!;
      const box = text.getBoundingClientRect();
      const ground = groundAt(box.x + box.width / 2, box.y + box.height / 2, svg, text);
      results.push({kind: 'text', what: text.textContent ?? '', ratio: ratio(over(paint(text, 'fill'), ground), ground)});
    });
    const walker = document.createTreeWalker(rootElement, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const parent = node.parentElement!;
      if (!node.textContent?.trim() || parent.closest('svg') || !visible(parent)) continue;
      const ground = htmlGround(parent);
      results.push({kind: 'text', what: node.textContent.trim(), ratio: ratio(over(paint(parent, 'color'), ground), ground)});
    }

    rootElement.querySelectorAll('svg *').forEach((shape) => {
      if (!(shape instanceof SVGGeometryElement) || !visible(shape)) return;
      const style = getComputedStyle(shape);
      const width = parseFloat(style.strokeWidth);
      if (!style.stroke || style.stroke === 'none' || !(width > 0)) return;
      const svg = shape.ownerSVGElement!;
      const matrix = shape.getScreenCTM()!;
      const length = shape.getTotalLength();
      const colour = paint(shape, 'stroke');
      for (const at of [0.25, 0.5, 0.75]) {
        const p = shape.getPointAtLength(length * at);
        const q = shape.getPointAtLength(Math.min(length, length * at + 0.5));
        const r = shape.getPointAtLength(Math.max(0, length * at - 0.5));
        const tx = q.x - r.x;
        const ty = q.y - r.y;
        const norm = Math.hypot(tx, ty) || 1;
        const offset = width / 2 + 1.5;
        for (const side of [1, -1]) {
          const local = new DOMPoint(p.x - (ty / norm) * offset * side, p.y + (tx / norm) * offset * side).matrixTransform(matrix);
          if (local.x < 0 || local.y < 0 || local.x >= innerWidth || local.y >= innerHeight) continue;
          const ground = groundAt(local.x, local.y, svg, shape);
          results.push({kind: 'line', what: `${shape.tagName}.${shape.getAttribute('class') ?? shape.parentElement?.getAttribute('class')}`, ratio: ratio(over(colour, ground), ground)});
        }
      }
    });
    // Marks drawn as a fill alone: the bars of the suite and the travelling message, against what is beside them.
    rootElement.querySelectorAll('.inbox-pitch__bar-fill, .inbox-hero__token rect').forEach((mark) => {
      if (!visible(mark)) return;
      const box = mark.getBoundingClientRect();
      const svg = (mark as SVGElement).ownerSVGElement!;
      const ground = groundAt(box.right + 3, box.y + box.height / 2, svg, mark, false);
      results.push({kind: 'line', what: `fill ${mark.getAttribute('class') ?? mark.parentElement?.getAttribute('class')}`, ratio: ratio(over(paint(mark, 'fill'), ground), ground)});
    });
    rootElement.querySelectorAll('.inbox-hero, .inbox-pitch__card, .inbox-pitch__arrow-button, .inbox-pitch__dot span').forEach((node) => {
      if (!visible(node)) return;
      const ground = htmlGround(node.parentElement);
      results.push({kind: 'line', what: `border ${node.className}`, ratio: ratio(over(paint(node, 'borderTopColor'), ground), ground)});
    });
    return results;
  });
}

for (const theme of THEMES) {
  test(`hero figure and slides meet the contrast floor in the ${theme} theme`, async ({page}) => {
    await page.setViewportSize({width: 1440, height: 1000});
    await page.emulateMedia({reducedMotion: 'no-preference'});
    await page.clock.install();
    await open(page, theme);
    const report: Awaited<ReturnType<typeof contrast>> = [];
    const hero = page.getByTestId('inbox-hero');
    for (let i = 0; i < animation.frames.length; i++) {
      report.push(...(await contrast(hero)));
      await page.clock.runFor(animation.frame_ms);
    }
    await page.emulateMedia({reducedMotion: 'reduce'});
    await expect(hero).toHaveAttribute('data-mode', 'still');
    report.push(...(await contrast(hero)));
    const pitch = page.getByTestId('pitch');
    for (const width of [1440, 390]) {
      await page.setViewportSize({width, height: 1000});
      if (width === 390) {
        await page.evaluate(() => window.scrollTo(0, 0));
        await expect(hero).toHaveAttribute('data-layout', 'tall');
        await hero.scrollIntoViewIfNeeded();
        report.push(...(await contrast(hero)));
      }
      for (const label of LABELS) {
        await pitch.getByRole('button', {name: `Show ${label}`}).click();
        await expect(activeSlide(page).getByTestId('pitch-label')).toHaveText(label);
        await page.mouse.move(1, 1);
        await pitch.scrollIntoViewIfNeeded();
        report.push(...(await contrast(pitch)));
      }
    }
    const texts = report.filter((r) => r.kind === 'text');
    const lines = report.filter((r) => r.kind === 'line');
    expect(texts.length).toBeGreaterThan(50);
    expect(lines.length).toBeGreaterThan(50);
    const lowest = (rows: typeof report) => rows.reduce((a, b) => (b.ratio < a.ratio ? b : a));
    const text = lowest(texts);
    const line = lowest(lines);
    console.log(`contrast ${theme}: lowest text ${text.ratio.toFixed(2)} (${text.what}), lowest line ${line.ratio.toFixed(2)} (${line.what}); ${texts.length} texts, ${lines.length} lines measured`);
    expect(texts.filter((r) => r.ratio < 4.5), 'text under 4.5:1').toEqual([]);
    expect(lines.filter((r) => r.ratio < 3), 'line under 3:1').toEqual([]);
  });
}

const DOCS = join(__dirname, '..', 'docs');
const LIMITS = readFileSync(join(DOCS, 'limits.md'), 'utf8');
const openFindings = () => (LIMITS.split('## Security findings, open')[1].split(/\n## /)[0].match(/^### \d+\./gm) ?? []).length;
const NUMBER_WORDS = ['zero', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine'];

test.describe('pitch slides', () => {
  test('are one slider directly below the hero and above the sections', async ({page}) => {
    await open(page);
    await expect(page.getByTestId('pitch')).toHaveCount(1);
    const order = await page.locator('.inbox-landing').evaluate((landing) =>
      Array.from(landing.children).map((child) =>
        child.matches('[data-testid=pitch]') ? 'pitch' : child.querySelector('[data-testid=inbox-hero]') ? 'hero' : child.tagName.toLowerCase(),
      ),
    );
    expect(order.slice(0, 2)).toEqual(['hero', 'pitch']);
    expect(order.slice(2).length).toBe(product.sections.length);
  });

  test('holds 3 slides, each a claim, one sentence, a graphic and a link, no number before a label', async ({page}) => {
    await open(page);
    const slides = page.getByTestId('pitch-slide');
    await expect(slides).toHaveCount(3);
    for (let i = 0; i < 3; i++) {
      const slide = slides.nth(i);
      const label = (await slide.getByTestId('pitch-label').textContent())!.trim();
      expect(label).toBe(LABELS[i]);
      expect(label, 'no number prefix').not.toMatch(/^\s*\d/);
      const claim = (await slide.getByTestId('pitch-claim').textContent())!.trim();
      const line = (await slide.getByTestId('pitch-line').textContent())!.trim();
      expect(words(claim), claim).toBeLessThanOrEqual(8);
      expect(words(line), line).toBeLessThanOrEqual(20);
      expect(line.match(/[.!?](\s|$)/g)?.length, line).toBe(1);
      await expect(slide.locator('svg')).toHaveCount(1);
      await expect(slide.getByTestId('pitch-link')).toHaveCount(1);
    }
    const names = await page.getByTestId('pitch').locator('[aria-roledescription=slide]').evaluateAll((nodes) => nodes.map((n) => n.getAttribute('aria-label') ?? ''));
    for (const name of names) expect(name, 'no number prefix').not.toMatch(/^\s*\d/);
    await expect(slides.nth(2).getByTestId('pitch-link')).toHaveAttribute('href', '/inbox/docs/limits');
  });

  test('slide 3 counts the open security findings of the limits page and no scenario', async ({page}) => {
    await open(page);
    const text = (await page.getByTestId('pitch-slide').nth(2).textContent())!;
    expect(text).toContain(`${NUMBER_WORDS[openFindings()]} security findings stay open`);
    expect(text).toContain(`${openFindings()} open`);
    expect(text).not.toMatch(/\d+ (of \d+|scenarios|witnessed)/);
  });

  test('moves with the arrows, the dots, the arrow keys and a swipe', async ({page}) => {
    await open(page);
    const pitch = page.getByTestId('pitch');
    const active = () => activeSlide(page).getByTestId('pitch-label');
    await expect(active()).toHaveText(LABELS[0]);
    await pitch.getByRole('button', {name: 'Next slide'}).click();
    await expect(active()).toHaveText(LABELS[1]);
    await pitch.getByRole('button', {name: 'Previous slide'}).click();
    await expect(active()).toHaveText(LABELS[0]);
    await pitch.getByRole('button', {name: `Show ${LABELS[2]}`}).click();
    await expect(active()).toHaveText(LABELS[2]);
    await pitch.getByTestId('pitch-viewport').focus();
    await page.keyboard.press('ArrowLeft');
    await expect(active()).toHaveText(LABELS[1]);
    await page.keyboard.press('ArrowRight');
    await expect(active()).toHaveText(LABELS[2]);
    await page.keyboard.press('ArrowRight');
    await expect(active()).toHaveText(LABELS[0]);
    const box = (await pitch.getByTestId('pitch-viewport').boundingBox())!;
    await page.mouse.move(box.x + box.width * 0.8, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.3, box.y + box.height / 2, {steps: 6});
    await page.mouse.up();
    await expect(active()).toHaveText(LABELS[1]);
    await page.mouse.move(box.x + box.width * 0.3, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.8, box.y + box.height / 2, {steps: 6});
    await page.mouse.up();
    await expect(active()).toHaveText(LABELS[0]);
  });

  test('advances on its own, and stops on hover and on focus', async ({page}) => {
    await page.emulateMedia({reducedMotion: 'no-preference'});
    await page.clock.install();
    await open(page);
    const pitch = page.getByTestId('pitch');
    const active = () => activeSlide(page).getByTestId('pitch-label');
    await expect(active()).toHaveText(LABELS[0]);
    await page.clock.runFor(8_000);
    await expect(active()).toHaveText(LABELS[1]);
    await pitch.hover();
    await page.clock.runFor(30_000);
    await expect(active()).toHaveText(LABELS[1]);
    await page.mouse.move(2, 2);
    await pitch.getByTestId('pitch-viewport').focus();
    await page.clock.runFor(30_000);
    await expect(active()).toHaveText(LABELS[1]);
  });

  test('does not advance under reduced motion', async ({page}) => {
    await page.emulateMedia({reducedMotion: 'reduce'});
    await page.clock.install();
    await open(page);
    await page.mouse.move(2, 2);
    await page.clock.runFor(30_000);
    await expect(activeSlide(page).getByTestId('pitch-label')).toHaveText(LABELS[0]);
  });

  test(`texts are at least ${MIN_PX} px at 1440`, async ({page}) => {
    await open(page);
    for (const label of LABELS) {
      await page.getByTestId('pitch').getByRole('button', {name: `Show ${label}`}).click();
      await page.mouse.move(1, 1);
      for (const size of await textSizes(activeSlide(page))) expect(size.px, size.text).toBeGreaterThanOrEqual(MIN_PX);
    }
  });
});

test.describe('at 390 px', () => {
  test.use({viewport: {width: 390, height: 844}});

  test('the page does not scroll sideways, and the figure and every slide are whole and readable', async ({page}) => {
    await page.emulateMedia({reducedMotion: 'reduce'});
    await open(page, 'dark');
    const width = await page.evaluate(() => ({scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth}));
    expect(width.scroll).toBeLessThanOrEqual(width.client);
    const hero = page.getByTestId('inbox-hero');
    await expect(hero).toHaveAttribute('data-layout', 'tall');
    await expect(page.getByTestId('hero-planned').locator('text', {hasText: animation.layout.planned})).toBeVisible();
    const inside = async (root: Locator, frame: Locator) => {
      const outer = (await frame.boundingBox())!;
      expect(outer.x).toBeGreaterThanOrEqual(0);
      expect(outer.x + outer.width).toBeLessThanOrEqual(390);
      const boxes = await root.evaluate((element) => {
        const out: {text: string; left: number; right: number}[] = [];
        element.querySelectorAll('svg text, p, h2, a').forEach((node) => {
          if (node.closest('[aria-hidden=true]:not(text)')) return;
          const box = node.getBoundingClientRect();
          if (box.width === 0) return;
          out.push({text: node.textContent ?? '', left: box.left, right: box.right});
        });
        return out;
      });
      expect(boxes.length).toBeGreaterThan(2);
      for (const box of boxes) {
        expect(box.left, box.text).toBeGreaterThanOrEqual(outer.x);
        expect(box.right, box.text).toBeLessThanOrEqual(outer.x + outer.width);
      }
    };
    await inside(hero, hero);
    for (const size of await textSizes(hero)) expect(size.px, size.text).toBeGreaterThanOrEqual(MIN_PX);
    const pitch = page.getByTestId('pitch');
    for (const label of LABELS) {
      await pitch.getByRole('button', {name: `Show ${label}`}).click();
      await expect(activeSlide(page).getByTestId('pitch-label')).toHaveText(label);
      await inside(activeSlide(page), pitch.locator('.inbox-pitch__card'));
      for (const size of await textSizes(activeSlide(page))) expect(size.px, size.text).toBeGreaterThanOrEqual(MIN_PX);
    }
  });
});

test.describe('claims', () => {
  test('the lede counts the open security findings of the limits page and claims one host', async () => {
    const findings = openFindings();
    expect(findings).toBe(2);
    expect(product.product.lede).toContain(`${NUMBER_WORDS[findings]} security findings are open`);
    expect(product.product.lede).toContain('one host');
    expect(product.product.lede).toContain('loopback only');
    expect(LIMITS).toContain('No second host');
  });

  test('nothing on the landing page says that 2 hosts work', async ({page}) => {
    await open(page);
    const text = (await page.locator('main').innerText()).toLowerCase();
    expect(text).not.toMatch(/(two|2|second|several|multiple|many|across) hosts?\b/);
    expect(text).not.toMatch(/on different hosts/);
  });
});
