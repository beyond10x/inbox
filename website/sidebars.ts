import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['index', 'first-use', 'status', 'limits'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: ['reference/cli', 'reference/http-api', 'reference/events'],
    },
    {
      type: 'category',
      label: 'Background',
      collapsed: false,
      items: ['design', 'how-it-is-built'],
    },
    {
      type: 'category',
      label: 'Project',
      collapsed: false,
      items: ['releases'],
    },
    {
      type: 'link',
      label: 'Specification',
      href: '/specification/',
    },
  ],
};

export default sidebars;
