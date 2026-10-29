// @ts-check
// Where the documentation's sources live. Every link to GitHub is built here.

export const REPOSITORY = 'https://github.com/dew-desktop/dew';
const BRANCH = 'main';

/**
 * The repository path of a page's source, from the path Astro's loader recorded,
 * which is relative to `site/`, or `undefined` for a page the site composes.
 *
 * @param {string | undefined} filePath
 */
export function sourcePath(filePath) {
	const path = filePath?.replace(/\\/g, '/').replace(/^(\.\.\/)+/, '');
	// Only a page from `docs/` has a source a reader can open.
	return path?.startsWith('docs/') ? path : undefined;
}

/** @param {string} path */
export const editUrl = (path) => `${REPOSITORY}/edit/${BRANCH}/${path}`;

/** @param {string} path */
export const rawUrl = (path) => `${REPOSITORY}/raw/${BRANCH}/${path}`;

/** @param {string} title */
export const issueUrl = (title) => `${REPOSITORY}/issues/new?title=${encodeURIComponent(title)}`;

export const releasesUrl = `${REPOSITORY}/releases`;
