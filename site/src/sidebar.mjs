// @ts-check
import { readdirSync, readFileSync } from 'node:fs';

export const DOCS = new URL('../../docs/', import.meta.url);

/**
 * A sidebar group listing every page in one directory of `docs/`.
 *
 * Starlight's `autogenerate` cannot do this: it matches pages by their path
 * under `src/content/docs/`, and these pages live outside the site. A page named
 * by slug is looked up by id instead, and takes its label from its own title.
 * Pages sort by `sidebar.order` in their frontmatter, then by file name.
 *
 * @param {string} label
 * @param {string} directory
 */
export function group(label, directory) {
	const pages = readdirSync(new URL(`${directory}/`, DOCS))
		.filter((name) => name.endsWith('.md') && !name.startsWith('_'))
		.map((name) => ({ name, order: order(new URL(`${directory}/${name}`, DOCS)) }))
		.sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
	return { label, items: pages.map(({ name }) => ({ slug: `${directory}/${name.replace(/\.md$/, '')}` })) };
}

/** @param {URL} file */
function order(file) {
	const frontmatter = /^---\r?\n([\s\S]*?)\r?\n---/.exec(readFileSync(file, 'utf8'))?.[1] ?? '';
	const sidebar = /^sidebar:\r?\n((?:[ \t]+.*\r?\n?)*)/m.exec(frontmatter)?.[1] ?? '';
	const value = /^[ \t]+order:[ \t]*(-?\d+)/m.exec(sidebar)?.[1];
	return value === undefined ? Number.POSITIVE_INFINITY : Number(value);
}
