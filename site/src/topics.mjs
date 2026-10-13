// @ts-check
// THE TOPICS, in the order the topic bar shows them. Each is a directory of
// `docs/`; `groups` lists its subdirectories in order, and `description` is the
// line the home page shows under it. Read by the sidebar in `astro.config.mjs`
// and by the home page, so the two cannot disagree.
//
// A topic is an area of what someone can build or use. Applets are one kind of
// Dew app; what every kind shares, such as the interface classes, is a topic of
// its own rather than a part of Applets.

/** @type {(import('./sidebar.mjs').Topic & { description: string })[]} */
export const TOPICS = [
	{
		label: 'Get started',
		directory: 'get-started',
		description: 'Build and run your first applet.',
	},
	{
		label: 'Applets',
		directory: 'applets',
		description: 'Apps that people install and run on their desktop.',
	},
	{
		label: 'Interface',
		directory: 'interface',
		description: 'Build interfaces with the classes you know from Roblox.',
	},
	{
		label: 'Specification',
		directory: 'spec',
		description: 'The services every host that runs Dew apps provides.',
	},
];
