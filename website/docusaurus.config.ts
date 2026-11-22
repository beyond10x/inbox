import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import {withProductSite} from '@beyond10x/docs-system/product-site';

const config: Config = {
  title: 'inbox',
  tagline: 'One durable, addressable inbox per agent or person, specified before any code.',
  favicon: 'img/logo.svg',

  future: {v4: true},
  url: 'https://beyond10x.github.io',
  baseUrl: '/inbox/',
  organizationName: 'beyond10x',
  projectName: 'inbox',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,
  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',

  markdown: {
    format: 'detect',
    mermaid: true,
    hooks: {onBrokenMarkdownLinks: 'throw'},
  },
  themes: ['@docusaurus/theme-mermaid'],
  i18n: {defaultLocale: 'en', locales: ['en']},

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: 'https://github.com/beyond10x/inbox/tree/main/website/',
        },
        blog: false,
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'specification',
        path: '../generated/docs/docs',
        routeBasePath: 'specification',
        sidebarPath: './sidebars-specification.ts',
      },
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'inbox',
      logo: {alt: 'inbox', src: 'img/logo.svg'},
      items: [
        {to: '/docs/', label: 'Docs', position: 'left', activeBaseRegex: '^/inbox/docs/(?!status|limits)'},
        {to: '/docs/status', label: 'Features', position: 'left'},
        {to: '/docs/limits', label: 'Limits', position: 'left'},
        {to: '/specification/', label: 'Specification', position: 'left', activeBaseRegex: '^/inbox/specification/'},
        {href: 'https://github.com/beyond10x/inbox', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      links: [
        {
          title: 'Documentation',
          items: [
            {label: 'What the inbox is', to: '/docs/'},
            {label: 'Features', to: '/docs/status'},
            {label: 'Limits', to: '/docs/limits'},
            {label: 'Design notes', to: '/docs/design'},
            {label: 'How it is built', to: '/docs/how-it-is-built'},
            {label: 'Specification', to: '/specification/'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'Releases', to: '/docs/releases'},
            {label: 'Changelog', href: 'https://github.com/beyond10x/inbox/blob/main/CHANGELOG.md'},
            {label: 'Source', href: 'https://github.com/beyond10x/inbox'},
          ],
        },
      ],
      copyright: 'A beyond10x project · Apache-2.0 · built with Docusaurus and the docs-system product template.',
    },
  } satisfies Preset.ThemeConfig,
};

export default withProductSite(config, {mark: 'I'});
