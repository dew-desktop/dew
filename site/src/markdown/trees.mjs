// @ts-check
/**
 * A directory layout, drawn as one:
 *
 *     ```tree
 *     my-applet/
 *       dew.toml         # what the applet is and what it may reach
 *       my-applet.luau   # the entry module
 *     ```
 *
 * Two spaces of indent are one level. A name ending in `/` is a directory. Text
 * after `#` describes the entry, and `code` in it is rendered as code. On GitHub
 * the same block is a plain listing, which is still the layout.
 *
 * The layout of an applet's directory is the main instruction these docs give,
 * so it is drawn rather than left as text in a code block.
 */

/** @param {string} text */
const escape = (text) =>
	text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** @param {string} text */
const inline = (text) => escape(text).replace(/`([^`]+)`/g, '<code>$1</code>');

/**
 * @typedef {{ name: string, note: string, children: Entry[] }} Entry
 * @param {string} source
 */
function parse(source) {
	/** @type {Entry[]} */
	const root = [];
	/** @type {{ depth: number, children: Entry[] }[]} */
	const stack = [{ depth: -1, children: root }];
	for (const line of source.split(/\r?\n/)) {
		if (!line.trim()) continue;
		const depth = Math.floor((line.length - line.trimStart().length) / 2);
		const [, name = '', note = ''] = /^\s*(.*?)\s*(?:#\s*(.*))?$/.exec(line) ?? [];
		while (stack[stack.length - 1].depth >= depth) stack.pop();
		/** @type {Entry} */
		const entry = { name, note, children: [] };
		stack[stack.length - 1].children.push(entry);
		stack.push({ depth, children: entry.children });
	}
	return root;
}

/** @param {Entry[]} entries @returns {string} */
function render(entries) {
	return `<ul>${entries
		.map(({ name, note, children }) => {
			const kind = name.endsWith('/') ? 'dir' : 'file';
			return (
				`<li class="dew-tree-${kind}"><span class="dew-tree-name">${escape(name)}</span>` +
				(note ? `<span class="dew-tree-note">${inline(note)}</span>` : '') +
				(children.length ? render(children) : '') +
				'</li>'
			);
		})
		.join('')}</ul>`;
}

/** @type {import('satteri').MdastPluginDefinition} */
export const trees = {
	name: 'dew-trees',
	code(node, ctx) {
		if (node.lang !== 'tree') return;
		ctx.replaceNode(node, {
			raw: `<figure class="dew-tree" aria-label="Directory layout">${render(parse(node.value))}</figure>`,
			mdxExpressions: false,
		});
	},
};
