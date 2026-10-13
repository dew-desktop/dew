// @ts-check
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const DOCS = fileURLToPath(new URL('../../../docs/', import.meta.url));
const RELATIVE_MARKDOWN = /^(?![a-z][a-z0-9+.-]*:|\/|#)([^#?]+\.mdx?)(#.*)?$/i;

/**
 * Rewrites a relative link to another Markdown file into that page's route.
 *
 * The files in `docs/` link to each other by path, `[scope](datamodel_scope.md)`,
 * which is what GitHub and an editor follow. A route is what the site serves, and
 * the two cannot both be written in the source. A link that leaves `docs/` is left
 * alone, so a broken one stays visible rather than becoming a plausible route.
 *
 * @param {{ fileURL: URL | undefined }} context
 */
export function relativeLinks({ fileURL }) {
	if (!fileURL) return null;
	const from = path.dirname(fileURLToPath(fileURL));

	/** @param {string} url */
	const route = (url) => {
		const match = RELATIVE_MARKDOWN.exec(url);
		if (!match) return undefined;
		const target = path.relative(DOCS, path.resolve(from, match[1]));
		if (target.startsWith('..') || path.isAbsolute(target)) return undefined;
		const slug = target
			.replace(/\\/g, '/')
			.replace(/\.mdx?$/i, '')
			.replace(/(^|\/)index$/, '');
		return `/${slug}${slug ? '/' : ''}${match[2] ?? ''}`;
	};

	/** @param {any} node @param {any} ctx */
	const rewrite = (node, ctx) => {
		const url = route(node.url);
		if (url !== undefined) ctx.setProperty(node, 'url', url);
	};

	return { name: 'dew-relative-links', link: rewrite, definition: rewrite };
}
