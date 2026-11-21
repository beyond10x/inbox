import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  specificationSidebar: [
    {
      type: 'category',
      label: 'Specification',
      collapsed: false,
      items: [
        {type: 'doc', id: 'index', label: 'Overview'},
        {type: 'doc', id: 'domains/inbox-messaging', label: 'Messaging'},
        {type: 'doc', id: 'interactions', label: 'Interactions'},
        {type: 'doc', id: 'crossings', label: 'Type crossings'},
        {type: 'doc', id: 'topology', label: 'Topology'},
      ],
    },
    {
      type: 'link',
      label: 'Back to the documentation',
      href: '/docs/',
    },
  ],
};

export default sidebars;
