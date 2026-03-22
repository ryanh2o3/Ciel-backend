/** Paths use trailing slashes to match `trailingSlash: true` and `usePathname()`. */
export const navigation = [
  {
    title: 'Introduction',
    links: [
      { title: 'Getting started', href: '/' },
      { title: 'Overview', href: '/docs/overview/' },
      { title: 'Local development', href: '/docs/local-development/' },
    ],
  },
  {
    title: 'Architecture',
    links: [
      { title: 'System architecture', href: '/docs/architecture/' },
      { title: 'Backend structure', href: '/docs/backend-structure/' },
      { title: 'Data and migrations', href: '/docs/data-and-migrations/' },
    ],
  },
  {
    title: 'Platform',
    links: [
      {
        title: 'Components and integrations',
        href: '/docs/platform-components/',
      },
    ],
  },
  {
    title: 'Clients',
    links: [
      { title: 'iOS app', href: '/docs/client-ios/' },
      { title: 'Android app', href: '/docs/client-android/' },
    ],
  },
  {
    title: 'API reference',
    links: [
      { title: 'Health and metrics', href: '/docs/api-health/' },
      { title: 'Auth and invites', href: '/docs/api-auth/' },
      { title: 'Users and account', href: '/docs/api-users/' },
      { title: 'Posts', href: '/docs/api-posts/' },
      { title: 'Feed', href: '/docs/api-feed/' },
      { title: 'Media', href: '/docs/api-media/' },
      { title: 'Notifications', href: '/docs/api-notifications/' },
      { title: 'Moderation and admin', href: '/docs/api-moderation/' },
      { title: 'Search', href: '/docs/api-search/' },
      { title: 'Stories', href: '/docs/api-stories/' },
      { title: 'Safety and invites', href: '/docs/api-safety/' },
    ],
  },
  {
    title: 'Operations',
    links: [
      { title: 'Scaling', href: '/docs/scaling/' },
      { title: 'Limitations', href: '/docs/limitations/' },
      { title: 'Docs deployment', href: '/docs/docs-deployment/' },
    ],
  },
]
