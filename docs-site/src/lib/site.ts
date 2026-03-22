/** Public URLs baked in at build time (optional override via env). */
export const productSiteUrl =
  process.env.NEXT_PUBLIC_PRODUCT_SITE_URL ?? 'https://ciel-social.eu'

export const sourceRepositoryUrl =
  process.env.NEXT_PUBLIC_GITHUB_REPO_URL ?? 'https://github.com'
