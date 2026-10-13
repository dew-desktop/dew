// @ts-check
/**
 * On a task page every H2 is a numbered step, except the sections that close the
 * page: "Next steps" and "See also" point onward and are not part of the task.
 * They are marked here, since CSS cannot read a heading's text.
 */

const CLOSING = new Set(['next steps', 'see also']);

/** @type {import('satteri').MdastPluginDefinition} */
export const steps = {
	name: 'dew-steps',
	heading(node, ctx) {
		if (node.depth !== 2 || !CLOSING.has(ctx.textContent(node).trim().toLowerCase())) return;
		ctx.setProperty(node, 'data', { hProperties: { className: ['dew-step-closing'] } });
	},
};
