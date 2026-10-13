// @ts-check
// THE TOPICS, in the order the topic bar shows them. Each is a directory of
// `docs/`; `groups` lists its subdirectories in order. Read by the sidebar in
// `astro.config.mjs` and by the home page, so the two cannot disagree.

/** @type {import('./sidebar.mjs').Topic[]} */
export const TOPICS = [
	{ label: 'Get started', directory: 'get-started' },
	{ label: 'Build applets', directory: 'build' },
	{ label: 'Reference', directory: 'reference' },
	{ label: 'Specification', directory: 'spec' },
];
