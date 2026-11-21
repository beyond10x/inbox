import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['index', 'status', 'limits'],
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
