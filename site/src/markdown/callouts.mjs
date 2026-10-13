// @ts-check
/**
 * GitHub's alert syntax, rendered as a callout:
 *
 *     > [!NOTE]
 *     > The manifest is read before any of the applet's code runs.
 *
 * The same source is a styled alert on GitHub and a callout on the site, so
 * `docs/` needs no component to have one. NOTE and TIP are notes; IMPORTANT,
 * WARNING and CAUTION are cautions. Anything else is left a plain quote.
 */

const KINDS = {
	NOTE: { title: 'Note', tone: 'note' },
	TIP: { title: 'Tip', tone: 'note' },
	IMPORTANT: { title: 'Important', tone: 'caution' },
	WARNING: { title: 'Warning', tone: 'caution' },
	CAUTION: { title: 'Caution', tone: 'caution' },
};

const MARKER = /^\[!([A-Z]+)\][ \t]*(?:\r?\n|$)/;

/** @type {import('satteri').MdastPluginDefinition} */
export const callouts = {
	name: 'dew-callouts',
	blockquote(node, ctx) {
		const first = node.children[0];
		if (first?.type !== 'paragraph') return;
		const text = first.children[0];
		if (text?.type !== 'text') return;
		const match = MARKER.exec(text.value);
		const kind = match && KINDS[/** @type {keyof typeof KINDS} */ (match[1])];
		if (!match || !kind) return;

		const rest = text.value.slice(match[0].length);
		if (rest) ctx.setProperty(text, 'value', rest);
		else if (first.children.length > 1) ctx.removeNode(text);
		else ctx.removeNode(first);

		ctx.setProperty(node, 'data', {
			hName: 'aside',
			hProperties: { className: ['dew-callout', `dew-callout--${kind.tone}`], ariaLabel: kind.title },
		});
		ctx.prependChild(node, {
			type: 'paragraph',
			data: { hProperties: { className: ['dew-callout-title'] } },
			children: [{ type: 'text', value: kind.title }],
		});
	},
};
