// @ts-check
import { existsSync, readdirSync, readFileSync } from 'node:fs';

export const DOCS = new URL('../../docs/', import.meta.url);

/**
 * @typedef {object} Topic
 * @property {string} label    What the topic bar and the breadcrumb call it.
 * @property {string} directory A directory of `docs/`.
 * @property {string[]} [groups] Its subdirectories, in order. Each becomes a
 *   sidebar group labelled from its name: `in-person` is "In person".
 */

/**
 * The sidebar: one group per topic, each holding the topic's own pages and then
 * one group per subdirectory. The route middleware shows only the topic the
 * current page is in, which is what makes a topic a Stripe-style tab.
 *
 * Starlight's `autogenerate` cannot build this: it matches pages by their path
 * under `src/content/docs/`, and these pages live outside the site. A page named
 * by slug is looked up by id instead, and takes its label from its own title.
 * Pages sort by `sidebar.order` in their frontmatter, then by file name.
 *
 * A subdirectory that is not listed in `groups` fails the build, so a page cannot
 * exist without being reachable. A topic with no pages yet is left out.
 *
 * @param {Topic[]} topics
 */
export function sidebar(topics) {
	return topics
		.filter(({ directory }) => existsSync(new URL(`${directory}/`, DOCS)))
		.map(({ label, directory, groups = [] }) => {
			const found = subdirectories(directory);
			const unlisted = found.filter((name) => !groups.includes(name));
			if (unlisted.length) {
				throw new Error(
					`docs/${directory}/ has ${unlisted.map((n) => `${n}/`).join(', ')}, which the ` +
						`'${label}' topic does not list in its groups (site/astro.config.mjs).`,
				);
			}
			const items = [
				...pages(directory),
				...groups
					.filter((name) => found.includes(name))
					.map((name) => ({ label: sentenceCase(name), items: pages(`${directory}/${name}`) })),
			];
			return { label, items };
		})
		.filter(({ items }) => items.length > 0);
}

/** @param {string} directory */
function subdirectories(directory) {
	return readdirSync(new URL(`${directory}/`, DOCS), { withFileTypes: true })
		.filter((entry) => entry.isDirectory())
		.map((entry) => entry.name);
}

/** @param {string} directory */
function pages(directory) {
	return readdirSync(new URL(`${directory}/`, DOCS))
		.filter((name) => name.endsWith('.md') && !name.startsWith('_'))
		.map((name) => ({ name, order: order(new URL(`${directory}/${name}`, DOCS)) }))
		.sort((a, b) => a.order - b.order || a.name.localeCompare(b.name))
		.map(({ name }) => ({ slug: `${directory}/${name.replace(/\.md$/, '')}` }));
}

/** @param {string} name */
function sentenceCase(name) {
	const words = name.replace(/-/g, ' ');
	return words.charAt(0).toUpperCase() + words.slice(1);
}

/** @param {URL} file */
function order(file) {
	const frontmatter = /^---\r?\n([\s\S]*?)\r?\n---/.exec(readFileSync(file, 'utf8'))?.[1] ?? '';
	const block = /^sidebar:\r?\n((?:[ \t]+.*\r?\n?)*)/m.exec(frontmatter)?.[1] ?? '';
	const value = /^[ \t]+order:[ \t]*(-?\d+)/m.exec(block)?.[1];
	return value === undefined ? Number.POSITIVE_INFINITY : Number(value);
}
