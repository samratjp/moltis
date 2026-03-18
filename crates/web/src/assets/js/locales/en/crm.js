// ── CRM page English strings ──────────────────────────────────

export default {
	// ── Page ─────────────────────────────────────────────────
	title: "CRM",

	// ── Contact list ─────────────────────────────────────────
	contacts: {
		title: "Contacts",
		add: "Add Contact",
		search: "Search contacts\u2026",
		all: "All",
		empty: "No contacts yet",
		emptySearch: "No contacts match your search",
		emptyHint: "Contacts are created automatically when someone messages you, or you can add one manually.",
		deleteConfirm: "Delete contact \u201c{{name}}\u201d? This cannot be undone.",
		stages: {
			lead: "Lead",
			prospect: "Prospect",
			active: "Active",
			inactive: "Inactive",
			closed: "Closed",
		},
	},

	// ── Contact detail ────────────────────────────────────────
	detail: {
		back: "Back",
		overview: "Overview",
		matters: "Matters",
		interactions: "Interactions",
		channels: "Channels",
		edit: "Edit",
		delete: "Delete",
		email: "Email",
		phone: "Phone",
		source: "Source",
		externalId: "External ID",
		stage: "Stage",
		notFound: "Contact not found",
	},

	// ── Matters ───────────────────────────────────────────────
	matters: {
		title: "Matters",
		add: "Add Matter",
		empty: "No matters yet",
		status: {
			open: "Open",
			on_hold: "On Hold",
			closed: "Closed",
			archived: "Archived",
		},
		phase: {
			intake: "Intake",
			discovery: "Discovery",
			negotiation: "Negotiation",
			resolution: "Resolution",
			review: "Review",
			closed: "Closed",
		},
		practiceArea: {
			corporate: "Corporate",
			employment: "Employment",
			family_law: "Family Law",
			immigration: "Immigration",
			intellectual_property: "IP",
			litigation: "Litigation",
			real_estate: "Real Estate",
			tax: "Tax",
			other: "Other",
		},
		deleteConfirm: "Delete matter \u201c{{title}}\u201d?",
	},

	// ── Interactions ──────────────────────────────────────────
	interactions: {
		title: "Interactions",
		add: "Record Interaction",
		empty: "No interactions recorded yet",
		kind: {
			call: "Call",
			email: "Email",
			message: "Message",
			meeting: "Meeting",
			note: "Note",
			document: "Document",
		},
		deleteConfirm: "Delete this interaction?",
	},

	// ── Channels (contact channels) ───────────────────────────
	channels: {
		empty: "No linked channels",
	},

	// ── Forms ─────────────────────────────────────────────────
	form: {
		// Contact form
		newContact: "New Contact",
		editContact: "Edit Contact",
		name: "Name",
		namePlaceholder: "Full name",
		nameRequired: "Name is required",
		email: "Email",
		emailPlaceholder: "email@example.com",
		phone: "Phone",
		phonePlaceholder: "+1 555 000 0000",
		stage: "Stage",
		source: "Source",
		sourcePlaceholder: "e.g. telegram, referral",
		externalId: "External ID",
		externalIdPlaceholder: "Platform user ID",

		// Matter form
		newMatter: "New Matter",
		editMatter: "Edit Matter",
		matterTitle: "Title",
		matterTitlePlaceholder: "e.g. Estate planning",
		matterTitleRequired: "Title is required",
		description: "Description",
		descriptionPlaceholder: "Optional notes",
		status: "Status",
		phase: "Phase",
		practiceArea: "Practice Area",

		// Interaction form
		recordInteraction: "Record Interaction",
		kind: "Type",
		summary: "Summary",
		summaryPlaceholder: "What happened?",
		summaryRequired: "Summary is required",
		matter: "Matter (optional)",
		noMatter: "No specific matter",

		// Common
		save: "Save",
		create: "Create",
		cancel: "Cancel",
	},

	// ── Errors ────────────────────────────────────────────────
	errors: {
		loadFailed: "Failed to load contacts",
		saveFailed: "Failed to save",
		deleteFailed: "Failed to delete",
		notFound: "Contact not found",
		loadMattersFailed: "Failed to load matters",
		loadInteractionsFailed: "Failed to load interactions",
		loadChannelsFailed: "Failed to load channels",
	},
};
