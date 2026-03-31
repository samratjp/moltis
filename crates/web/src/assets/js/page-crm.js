// ── CRM page (Preact + HTM + Signals) ────────────────────────

import { signal } from "@preact/signals";
import { html } from "htm/preact";
import { render } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";
import * as gon from "./gon.js";
import { sendRpc } from "./helpers.js";
import { useTranslation } from "./i18n.js";
import { updateNavCount } from "./nav-counts.js";
import { navigate, registerPrefix } from "./router.js";
import { routes } from "./routes.js";
import { ConfirmDialog, requestConfirm, showToast } from "./ui.js";

// ── Module-level signals ─────────────────────────────────────
var contacts = signal([]);
var loadingContacts = signal(true);
var searchQuery = signal("");
var stageFilter = signal("");
var currentContactId = signal(null);
var contactDetail = signal(null);
var loadingDetail = signal(false);
var matters = signal([]);
var interactions = signal([]);
var channels = signal([]);
var loadingMatters = signal(false);
var loadingInteractions = signal(false);
var loadingChannels = signal(false);
var detailTab = signal("overview");

// ── Enum label/color maps ─────────────────────────────────────

var STAGE_COLORS = {
	lead: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
	prospect: "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200",
	active: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
	inactive: "bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200",
	closed: "bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400",
};

var MATTER_STATUS_COLORS = {
	open: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
	on_hold: "bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200",
	closed: "bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400",
	archived: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-500",
};

var INTERACTION_KIND_ICONS = {
	call: "icon-phone",
	email: "icon-mail",
	message: "icon-chat-bubble",
	meeting: "icon-users",
	note: "icon-file",
	document: "icon-file",
};

// ── Helpers ───────────────────────────────────────────────────

function newId() {
	return crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function initials(name) {
	if (!name) return "?";
	var parts = name.trim().split(/\s+/);
	if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
	return name.slice(0, 2).toUpperCase();
}

var AVATAR_COLORS = [
	"bg-blue-500",
	"bg-purple-500",
	"bg-green-500",
	"bg-amber-500",
	"bg-rose-500",
	"bg-teal-500",
	"bg-indigo-500",
	"bg-orange-500",
];

function avatarColor(id) {
	var hash = 0;
	for (var i = 0; i < (id || "").length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) & 0xffffffff;
	}
	return AVATAR_COLORS[Math.abs(hash) % AVATAR_COLORS.length];
}

function formatDate(epochMs) {
	if (!epochMs) return "";
	return new Date(epochMs).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

function formatDateTime(epochMs) {
	if (!epochMs) return "";
	return new Date(epochMs).toLocaleString(undefined, {
		year: "numeric",
		month: "short",
		day: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	});
}

// Client-side filter for contacts list
function filterContacts(list, query, stage) {
	var q = (query || "").toLowerCase().trim();
	return list.filter((c) => {
		if (stage && c.stage !== stage) return false;
		if (!q) return true;
		var name = (c.name || "").toLowerCase();
		var email = (c.email || "").toLowerCase();
		var phone = (c.phone || "").toLowerCase();
		return name.includes(q) || email.includes(q) || phone.includes(q);
	});
}

// ── Data loading ──────────────────────────────────────────────

function loadContacts() {
	loadingContacts.value = true;
	sendRpc("crm.contacts.list", {}).then((res) => {
		loadingContacts.value = false;
		if (!res?.ok) {
			showToast("errors.loadFailed", "error");
			return;
		}
		var list = Array.isArray(res.payload) ? res.payload : [];
		contacts.value = list;
		updateNavCount("crm", list.length);
	});
}

function loadContactDetail(id) {
	loadingDetail.value = true;
	contactDetail.value = null;
	sendRpc("crm.contacts.get", { id }).then((res) => {
		loadingDetail.value = false;
		if (!(res?.ok && res.payload)) {
			navigate(routes.crm);
			return;
		}
		contactDetail.value = res.payload;
	});
}

function loadMatters(contactId) {
	loadingMatters.value = true;
	sendRpc("crm.matters.list", {}).then((res) => {
		loadingMatters.value = false;
		if (!res?.ok) return;
		var all = Array.isArray(res.payload) ? res.payload : [];
		matters.value = all.filter((m) => m.contactId === contactId);
	});
}

function loadInteractions(contactId) {
	loadingInteractions.value = true;
	sendRpc("crm.interactions.list", { contactId }).then((res) => {
		loadingInteractions.value = false;
		if (!res?.ok) return;
		interactions.value = Array.isArray(res.payload) ? res.payload : [];
	});
}

function loadChannels(contactId) {
	loadingChannels.value = true;
	sendRpc("crm.channels.list", { contactId }).then((res) => {
		loadingChannels.value = false;
		if (!res?.ok) return;
		channels.value = Array.isArray(res.payload) ? res.payload : [];
	});
}

// ── StageBadge component ──────────────────────────────────────
function StageBadge({ stage, t }) {
	var color = STAGE_COLORS[stage] || STAGE_COLORS.lead;
	var label = t(`contacts.stages.${stage}`) || stage;
	return html`<span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${color}">${label}</span>`;
}

// ── Avatar component ──────────────────────────────────────────
function Avatar({ name, id, size = "md" }) {
	var color = avatarColor(id || name || "");
	var sizeClass = size === "sm" ? "w-7 h-7 text-xs" : size === "lg" ? "w-12 h-12 text-lg" : "w-9 h-9 text-sm";
	return html`<div class="rounded-full ${color} ${sizeClass} flex items-center justify-center text-white font-medium flex-shrink-0">
		${initials(name)}
	</div>`;
}

// ── ContactForm modal ─────────────────────────────────────────
function ContactForm({ contact, onClose, onSaved }) {
	var { t } = useTranslation("crm");
	var isEdit = !!contact?.id;

	var [name, setName] = useState(contact?.name || "");
	var [email, setEmail] = useState(contact?.email || "");
	var [phone, setPhone] = useState(contact?.phone || "");
	var [stage, setStage] = useState(contact?.stage || "lead");
	var [source, setSource] = useState(contact?.source || "");
	var [externalId, setExternalId] = useState(contact?.externalId || "");
	var [saving, setSaving] = useState(false);
	var [error, setError] = useState(null);

	function onSubmit(e) {
		e.preventDefault();
		if (!name.trim()) {
			setError(t("form.nameRequired"));
			return;
		}
		setSaving(true);
		setError(null);
		var params = {
			id: contact?.id || newId(),
			name: name.trim(),
			email: email.trim() || null,
			phone: phone.trim() || null,
			stage,
			source: source.trim() || null,
			externalId: externalId.trim() || null,
			createdAt: contact?.createdAt,
		};
		sendRpc("crm.contacts.upsert", params).then((res) => {
			setSaving(false);
			if (!res?.ok) {
				setError(t("errors.saveFailed"));
				return;
			}
			onSaved();
		});
	}

	var stages = ["lead", "prospect", "active", "inactive", "closed"];

	return html`<form onSubmit=${onSubmit}>
		<div class="flex flex-col gap-3 mb-4">
			<div>
				<label class="block text-xs text-[var(--muted)] mb-1">${t("form.name")} *</label>
				<input
					type="text"
					value=${name}
					onInput=${(e) => setName(e.target.value)}
					placeholder=${t("form.namePlaceholder")}
					class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					autofocus
				/>
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.email")}</label>
					<input
						type="email"
						value=${email}
						onInput=${(e) => setEmail(e.target.value)}
						placeholder=${t("form.emailPlaceholder")}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					/>
				</div>
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.phone")}</label>
					<input
						type="tel"
						value=${phone}
						onInput=${(e) => setPhone(e.target.value)}
						placeholder=${t("form.phonePlaceholder")}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					/>
				</div>
			</div>
			<div>
				<label class="block text-xs text-[var(--muted)] mb-1">${t("form.stage")}</label>
				<select
					value=${stage}
					onChange=${(e) => setStage(e.target.value)}
					class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
				>
					${stages.map((s) => html`<option key=${s} value=${s}>${t(`contacts.stages.${s}`)}</option>`)}
				</select>
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.source")}</label>
					<input
						type="text"
						value=${source}
						onInput=${(e) => setSource(e.target.value)}
						placeholder=${t("form.sourcePlaceholder")}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					/>
				</div>
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.externalId")}</label>
					<input
						type="text"
						value=${externalId}
						onInput=${(e) => setExternalId(e.target.value)}
						placeholder=${t("form.externalIdPlaceholder")}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					/>
				</div>
			</div>
			${error && html`<p class="text-xs text-[var(--error)]">${error}</p>`}
		</div>
		<div class="flex gap-2 justify-end">
			<button type="button" class="provider-btn provider-btn-secondary" onClick=${onClose}>${t("form.cancel")}</button>
			<button type="submit" class="provider-btn" disabled=${saving}>${saving ? "\u2026" : isEdit ? t("form.save") : t("form.create")}</button>
		</div>
	</form>`;
}

// ── MatterForm modal ──────────────────────────────────────────
function MatterForm({ matter, contactId, onClose, onSaved }) {
	var { t } = useTranslation("crm");
	var isEdit = !!matter?.id;

	var [title, setTitle] = useState(matter?.title || "");
	var [description, setDescription] = useState(matter?.description || "");
	var [status, setStatus] = useState(matter?.status || "open");
	var [phase, setPhase] = useState(matter?.phase || "intake");
	var [practiceArea, setPracticeArea] = useState(matter?.practiceArea || "other");
	var [saving, setSaving] = useState(false);
	var [error, setError] = useState(null);

	function onSubmit(e) {
		e.preventDefault();
		if (!title.trim()) {
			setError(t("form.matterTitleRequired"));
			return;
		}
		setSaving(true);
		setError(null);
		var params = {
			id: matter?.id || newId(),
			title: title.trim(),
			description: description.trim() || null,
			contactId: contactId || matter?.contactId || null,
			status,
			phase,
			practiceArea,
			createdAt: matter?.createdAt,
		};
		sendRpc("crm.matters.upsert", params).then((res) => {
			setSaving(false);
			if (!res?.ok) {
				setError(t("errors.saveFailed"));
				return;
			}
			onSaved();
		});
	}

	var statuses = ["open", "on_hold", "closed", "archived"];
	var phases = ["intake", "discovery", "negotiation", "resolution", "review", "closed"];
	var practiceAreas = [
		"corporate",
		"employment",
		"family_law",
		"immigration",
		"intellectual_property",
		"litigation",
		"real_estate",
		"tax",
		"other",
	];

	return html`<form onSubmit=${onSubmit}>
		<div class="flex flex-col gap-3 mb-4">
			<div>
				<label class="block text-xs text-[var(--muted)] mb-1">${t("form.matterTitle")} *</label>
				<input
					type="text"
					value=${title}
					onInput=${(e) => setTitle(e.target.value)}
					placeholder=${t("form.matterTitlePlaceholder")}
					class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					autofocus
				/>
			</div>
			<div>
				<label class="block text-xs text-[var(--muted)] mb-1">${t("form.description")}</label>
				<textarea
					value=${description}
					onInput=${(e) => setDescription(e.target.value)}
					placeholder=${t("form.descriptionPlaceholder")}
					rows="2"
					class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)] resize-none"
				/>
			</div>
			<div class="grid grid-cols-3 gap-3">
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.status")}</label>
					<select
						value=${status}
						onChange=${(e) => setStatus(e.target.value)}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					>
						${statuses.map((s) => html`<option key=${s} value=${s}>${t(`matters.status.${s}`)}</option>`)}
					</select>
				</div>
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.phase")}</label>
					<select
						value=${phase}
						onChange=${(e) => setPhase(e.target.value)}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					>
						${phases.map((p) => html`<option key=${p} value=${p}>${t(`matters.phase.${p}`)}</option>`)}
					</select>
				</div>
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.practiceArea")}</label>
					<select
						value=${practiceArea}
						onChange=${(e) => setPracticeArea(e.target.value)}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					>
						${practiceAreas.map((a) => html`<option key=${a} value=${a}>${t(`matters.practiceArea.${a}`)}</option>`)}
					</select>
				</div>
			</div>
			${error && html`<p class="text-xs text-[var(--error)]">${error}</p>`}
		</div>
		<div class="flex gap-2 justify-end">
			<button type="button" class="provider-btn provider-btn-secondary" onClick=${onClose}>${t("form.cancel")}</button>
			<button type="submit" class="provider-btn" disabled=${saving}>${saving ? "\u2026" : isEdit ? t("form.save") : t("form.create")}</button>
		</div>
	</form>`;
}

// ── InteractionForm modal ─────────────────────────────────────
function InteractionForm({ contactId, onClose, onSaved }) {
	var { t } = useTranslation("crm");
	var matterList = matters.value;

	var [kind, setKind] = useState("note");
	var [summary, setSummary] = useState("");
	var [matterId, setMatterId] = useState("");
	var [saving, setSaving] = useState(false);
	var [error, setError] = useState(null);

	function onSubmit(e) {
		e.preventDefault();
		if (!summary.trim()) {
			setError(t("form.summaryRequired"));
			return;
		}
		setSaving(true);
		setError(null);
		var params = {
			id: newId(),
			contactId,
			kind,
			summary: summary.trim(),
			matterId: matterId || null,
		};
		sendRpc("crm.interactions.upsert", params).then((res) => {
			setSaving(false);
			if (!res?.ok) {
				setError(t("errors.saveFailed"));
				return;
			}
			onSaved();
		});
	}

	var kinds = ["call", "email", "message", "meeting", "note", "document"];

	return html`<form onSubmit=${onSubmit}>
		<div class="flex flex-col gap-3 mb-4">
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label class="block text-xs text-[var(--muted)] mb-1">${t("form.kind")}</label>
					<select
						value=${kind}
						onChange=${(e) => setKind(e.target.value)}
						class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
					>
						${kinds.map((k) => html`<option key=${k} value=${k}>${t(`interactions.kind.${k}`)}</option>`)}
					</select>
				</div>
				${
					matterList.length > 0 &&
					html`<div>
						<label class="block text-xs text-[var(--muted)] mb-1">${t("form.matter")}</label>
						<select
							value=${matterId}
							onChange=${(e) => setMatterId(e.target.value)}
							class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
						>
							<option value="">${t("form.noMatter")}</option>
							${matterList.map((m) => html`<option key=${m.id} value=${m.id}>${m.title}</option>`)}
						</select>
					</div>`
				}
			</div>
			<div>
				<label class="block text-xs text-[var(--muted)] mb-1">${t("form.summary")} *</label>
				<textarea
					value=${summary}
					onInput=${(e) => setSummary(e.target.value)}
					placeholder=${t("form.summaryPlaceholder")}
					rows="3"
					class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-2.5 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)] resize-none"
					autofocus
				/>
			</div>
			${error && html`<p class="text-xs text-[var(--error)]">${error}</p>`}
		</div>
		<div class="flex gap-2 justify-end">
			<button type="button" class="provider-btn provider-btn-secondary" onClick=${onClose}>${t("form.cancel")}</button>
			<button type="submit" class="provider-btn" disabled=${saving}>${saving ? "\u2026" : t("form.create")}</button>
		</div>
	</form>`;
}

// ── Modal wrapper (inline) ────────────────────────────────────
function CrmModal({ title, onClose, children }) {
	useEffect(() => {
		function onKey(e) {
			if (e.key === "Escape") onClose();
		}
		document.addEventListener("keydown", onKey);
		return () => document.removeEventListener("keydown", onKey);
	}, [onClose]);

	return html`<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		style="background:rgba(0,0,0,0.5)"
		onClick=${(e) => e.target === e.currentTarget && onClose()}
	>
		<div class="bg-[var(--surface)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-md max-h-[90vh] overflow-y-auto">
			<div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border)]">
				<h2 class="text-sm font-semibold text-[var(--text-strong)]">${title}</h2>
				<button
					type="button"
					class="text-[var(--muted)] hover:text-[var(--text)] bg-transparent border-0 cursor-pointer p-1 rounded"
					onClick=${onClose}
				>
					<span class="icon icon-sm icon-x"></span>
				</button>
			</div>
			<div class="p-4">${children}</div>
		</div>
	</div>`;
}

// ── Contact list page ─────────────────────────────────────────
function ContactListPage() {
	var { t } = useTranslation("crm");
	var searchRef = useRef(null);
	var debounceRef = useRef(null);

	var [localSearch, setLocalSearch] = useState(searchQuery.value);
	var [showAddForm, setShowAddForm] = useState(false);

	function onSearchInput(e) {
		var v = e.target.value;
		setLocalSearch(v);
		clearTimeout(debounceRef.current);
		debounceRef.current = setTimeout(() => {
			searchQuery.value = v;
		}, 300);
	}

	function onStageClick(s) {
		stageFilter.value = stageFilter.value === s ? "" : s;
	}

	function onContactClick(contactId) {
		navigate(`${routes.crm}/${contactId}`);
	}

	function onDeleteContact(e, contact) {
		e.stopPropagation();
		requestConfirm(t("contacts.deleteConfirm", { name: contact.name }), { danger: true }).then((yes) => {
			if (!yes) return;
			sendRpc("crm.contacts.delete", { id: contact.id }).then((res) => {
				if (!res?.ok) {
					showToast(t("errors.deleteFailed"), "error");
					return;
				}
				loadContacts();
			});
		});
	}

	var filtered = filterContacts(contacts.value, searchQuery.value, stageFilter.value);
	var stages = ["lead", "prospect", "active", "inactive", "closed"];

	return html`<div class="flex flex-col h-full">
		${
			showAddForm &&
			html`<${CrmModal} title=${t("form.newContact")} onClose=${() => setShowAddForm(false)}>
			<${ContactForm}
				contact=${null}
				onClose=${() => setShowAddForm(false)}
				onSaved=${() => {
					setShowAddForm(false);
					loadContacts();
				}}
			/>
		</${CrmModal}>`
		}

		<!-- Header -->
		<div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border)] shrink-0">
			<h1 class="text-base font-semibold text-[var(--text-strong)]">${t("contacts.title")}</h1>
			<button class="provider-btn" onClick=${() => setShowAddForm(true)}>
				${t("contacts.add")}
			</button>
		</div>

		<!-- Search + stage filter -->
		<div class="flex flex-col gap-2 px-4 py-2 border-b border-[var(--border)] shrink-0">
			<input
				ref=${searchRef}
				type="text"
				value=${localSearch}
				onInput=${onSearchInput}
				placeholder=${t("contacts.search")}
				class="w-full text-sm bg-[var(--surface2)] border border-[var(--border)] rounded px-3 py-1.5 text-[var(--text)] focus:outline-none focus:border-[var(--border-strong)]"
			/>
			<div class="flex gap-1.5 flex-wrap">
				<button
					class="text-xs px-2.5 py-1 rounded-full border transition-colors cursor-pointer ${stageFilter.value ? "border-[var(--border)] text-[var(--muted)] hover:border-[var(--border-strong)] hover:text-[var(--text)] bg-transparent" : "bg-[var(--accent)] border-[var(--accent)] text-white"}"
					onClick=${() => {
						stageFilter.value = "";
					}}
				>
					${t("contacts.all")}
					<span class="ml-1 font-medium">${contacts.value.length}</span>
				</button>
				${stages.map((s) => {
					var count = contacts.value.filter((c) => c.stage === s).length;
					var active = stageFilter.value === s;
					return html`<button
						key=${s}
						class="text-xs px-2.5 py-1 rounded-full border transition-colors cursor-pointer ${active ? "bg-[var(--accent)] border-[var(--accent)] text-white" : "border-[var(--border)] text-[var(--muted)] hover:border-[var(--border-strong)] hover:text-[var(--text)] bg-transparent"}"
						onClick=${() => onStageClick(s)}
					>
						${t(`contacts.stages.${s}`)}
						${count > 0 && html`<span class="ml-1 font-medium">${count}</span>`}
					</button>`;
				})}
			</div>
		</div>

		<!-- Contact list -->
		<div class="flex-1 overflow-y-auto">
			${
				loadingContacts.value
					? html`<div class="flex items-center justify-center py-16 text-[var(--muted)]">
							<span class="text-sm">Loading\u2026</span>
						</div>`
					: filtered.length === 0
						? html`<div class="flex flex-col items-center justify-center py-16 px-6 text-center">
								<div class="text-3xl mb-4">👥</div>
								<p class="text-sm font-medium text-[var(--text)] mb-2">
									${searchQuery.value || stageFilter.value ? t("contacts.emptySearch") : t("contacts.empty")}
								</p>
								${
									!(searchQuery.value || stageFilter.value) &&
									html`<p class="text-xs text-[var(--muted)] max-w-xs">${t("contacts.emptyHint")}</p>
										<button class="provider-btn mt-4" onClick=${() => setShowAddForm(true)}>
											${t("contacts.add")}
										</button>`
								}
							</div>`
						: html`<div class="divide-y divide-[var(--border)]">
								${filtered.map(
									(contact) => html`<div
										key=${contact.id}
										class="flex items-center gap-3 px-4 py-3 cursor-pointer hover:bg-[var(--bg-hover)] transition-colors group"
										onClick=${() => onContactClick(contact.id)}
									>
										<${Avatar} name=${contact.name} id=${contact.id} size="md" />
										<div class="flex-1 min-w-0">
											<div class="flex items-center gap-2">
												<span class="text-sm font-medium text-[var(--text-strong)] truncate">${contact.name}</span>
												<${StageBadge} stage=${contact.stage} t=${t} />
											</div>
											${
												(contact.email || contact.phone) &&
												html`<div class="flex items-center gap-3 mt-0.5">
													${contact.email && html`<span class="text-xs text-[var(--muted)] truncate">${contact.email}</span>`}
													${contact.phone && html`<span class="text-xs text-[var(--muted)]">${contact.phone}</span>`}
												</div>`
											}
										</div>
										<div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
											<button
												class="provider-btn provider-btn-sm provider-btn-danger"
												onClick=${(e) => onDeleteContact(e, contact)}
												title="Delete"
											>
												Delete
											</button>
										</div>
									</div>`,
								)}
							</div>`
			}
		</div>
		<${ConfirmDialog} />
	</div>`;
}

// ── Contact detail page ────────────────────────────────────────
function ContactDetailPage({ contactId }) {
	var { t } = useTranslation("crm");
	var contact = contactDetail.value;

	var [showEditForm, setShowEditForm] = useState(false);
	var [showAddMatter, setShowAddMatter] = useState(false);
	var [editingMatter, setEditingMatter] = useState(null);
	var [showAddInteraction, setShowAddInteraction] = useState(false);

	useEffect(() => {
		loadContactDetail(contactId);
		detailTab.value = "overview";
	}, [contactId]);

	useEffect(() => {
		if (!contact) return;
		loadMatters(contact.id);
		loadInteractions(contact.id);
		loadChannels(contact.id);
	}, [contact?.id]);

	function onDeleteContact() {
		if (!contact) return;
		requestConfirm(t("contacts.deleteConfirm", { name: contact.name }), { danger: true }).then((yes) => {
			if (!yes) return;
			sendRpc("crm.contacts.delete", { id: contact.id }).then((res) => {
				if (!res?.ok) {
					showToast(t("errors.deleteFailed"), "error");
					return;
				}
				loadContacts();
				navigate(routes.crm);
			});
		});
	}

	function onDeleteMatter(matter) {
		requestConfirm(t("matters.deleteConfirm", { title: matter.title }), { danger: true }).then((yes) => {
			if (!yes) return;
			sendRpc("crm.matters.delete", { id: matter.id }).then((res) => {
				if (!res?.ok) {
					showToast(t("errors.deleteFailed"), "error");
					return;
				}
				loadMatters(contactId);
			});
		});
	}

	function onDeleteInteraction(interaction) {
		requestConfirm(t("interactions.deleteConfirm"), { danger: true }).then((yes) => {
			if (!yes) return;
			sendRpc("crm.interactions.delete", { id: interaction.id }).then((res) => {
				if (!res?.ok) {
					showToast(t("errors.deleteFailed"), "error");
					return;
				}
				loadInteractions(contactId);
			});
		});
	}

	if (loadingDetail.value) {
		return html`<div class="flex items-center justify-center h-full text-[var(--muted)]">
			<span class="text-sm">Loading\u2026</span>
		</div>`;
	}

	if (!contact) {
		return html`<div class="flex flex-col items-center justify-center h-full gap-3">
			<p class="text-sm text-[var(--muted)]">${t("detail.notFound")}</p>
			<button class="provider-btn provider-btn-secondary" onClick=${() => navigate(routes.crm)}>
				${t("detail.back")}
			</button>
		</div>`;
	}

	var tab = detailTab.value;

	return html`<div class="flex flex-col h-full overflow-hidden">
		${
			showEditForm &&
			html`<${CrmModal} title=${t("form.editContact")} onClose=${() => setShowEditForm(false)}>
			<${ContactForm}
				contact=${contact}
				onClose=${() => setShowEditForm(false)}
				onSaved=${() => {
					setShowEditForm(false);
					loadContactDetail(contactId);
					loadContacts();
				}}
			/>
		</${CrmModal}>`
		}

		${
			showAddMatter &&
			html`<${CrmModal} title=${t("form.newMatter")} onClose=${() => setShowAddMatter(false)}>
			<${MatterForm}
				matter=${null}
				contactId=${contactId}
				onClose=${() => setShowAddMatter(false)}
				onSaved=${() => {
					setShowAddMatter(false);
					loadMatters(contactId);
				}}
			/>
		</${CrmModal}>`
		}

		${
			editingMatter &&
			html`<${CrmModal} title=${t("form.editMatter")} onClose=${() => setEditingMatter(null)}>
			<${MatterForm}
				matter=${editingMatter}
				contactId=${contactId}
				onClose=${() => setEditingMatter(null)}
				onSaved=${() => {
					setEditingMatter(null);
					loadMatters(contactId);
				}}
			/>
		</${CrmModal}>`
		}

		${
			showAddInteraction &&
			html`<${CrmModal} title=${t("form.recordInteraction")} onClose=${() => setShowAddInteraction(false)}>
			<${InteractionForm}
				contactId=${contactId}
				onClose=${() => setShowAddInteraction(false)}
				onSaved=${() => {
					setShowAddInteraction(false);
					loadInteractions(contactId);
				}}
			/>
		</${CrmModal}>`
		}

		<!-- Header -->
		<div class="flex items-start gap-3 px-4 py-3 border-b border-[var(--border)] shrink-0">
			<button
				class="text-[var(--muted)] hover:text-[var(--text)] bg-transparent border-0 cursor-pointer p-1 -ml-1 mt-0.5"
				onClick=${() => navigate(routes.crm)}
				title=${t("detail.back")}
			>
				<span class="icon icon-sm icon-chevron-left"></span>
			</button>
			<${Avatar} name=${contact.name} id=${contact.id} size="lg" />
			<div class="flex-1 min-w-0">
				<div class="flex items-center gap-2 flex-wrap">
					<h1 class="text-base font-semibold text-[var(--text-strong)]">${contact.name}</h1>
					<${StageBadge} stage=${contact.stage} t=${t} />
				</div>
				${
					(contact.email || contact.phone) &&
					html`<div class="flex items-center gap-4 mt-1 flex-wrap">
						${
							contact.email &&
							html`<span class="flex items-center gap-1 text-xs text-[var(--muted)]">
								<span class="icon icon-xs icon-mail"></span>
								<a href="mailto:${contact.email}" class="hover:text-[var(--accent)] hover:underline" onClick=${(e) => e.stopPropagation()}>${contact.email}</a>
							</span>`
						}
						${
							contact.phone &&
							html`<span class="flex items-center gap-1 text-xs text-[var(--muted)]">
								<span class="icon icon-xs icon-phone"></span>
								<a href="tel:${contact.phone}" class="hover:text-[var(--accent)] hover:underline" onClick=${(e) => e.stopPropagation()}>${contact.phone}</a>
							</span>`
						}
					</div>`
				}
			</div>
			<div class="flex gap-2 shrink-0">
				<button class="provider-btn provider-btn-sm provider-btn-secondary" onClick=${() => setShowEditForm(true)}>
					${t("detail.edit")}
				</button>
				<button class="provider-btn provider-btn-sm provider-btn-danger" onClick=${onDeleteContact}>
					${t("detail.delete")}
				</button>
			</div>
		</div>

		<!-- Tabs -->
		<div class="flex border-b border-[var(--border)] shrink-0 px-4">
			${["overview", "matters", "interactions", "channels"].map(
				(tabId) => html`<button
					key=${tabId}
					class="text-xs py-2 px-3 border-b-2 cursor-pointer bg-transparent transition-colors ${tab === tabId ? "border-[var(--accent)] text-[var(--accent)]" : "border-transparent text-[var(--muted)] hover:text-[var(--text)]"}"
					onClick=${() => {
						detailTab.value = tabId;
					}}
				>
					${t(`detail.${tabId}`)}
					${tabId === "matters" && matters.value.length > 0 && html`<span class="ml-1 text-[var(--muted)]">${matters.value.length}</span>`}
					${tabId === "interactions" && interactions.value.length > 0 && html`<span class="ml-1 text-[var(--muted)]">${interactions.value.length}</span>`}
				</button>`,
			)}
		</div>

		<!-- Tab content -->
		<div class="flex-1 overflow-y-auto">
			${tab === "overview" && html`<${OverviewTab} contact=${contact} t=${t} />`}
			${tab === "matters" && html`<${MattersTab} contactId=${contactId} t=${t} onAdd=${() => setShowAddMatter(true)} onEdit=${setEditingMatter} onDelete=${onDeleteMatter} />`}
			${tab === "interactions" && html`<${InteractionsTab} contactId=${contactId} t=${t} onAdd=${() => setShowAddInteraction(true)} onDelete=${onDeleteInteraction} />`}
			${tab === "channels" && html`<${ChannelsTab} t=${t} />`}
		</div>
		<${ConfirmDialog} />
	</div>`;
}

// ── Overview tab ──────────────────────────────────────────────
function OverviewTab({ contact, t }) {
	var rows = [
		contact.email && { label: t("detail.email"), value: contact.email },
		contact.phone && { label: t("detail.phone"), value: contact.phone },
		contact.source && { label: t("detail.source"), value: contact.source },
		contact.externalId && { label: t("detail.externalId"), value: contact.externalId },
		{ label: t("detail.stage"), value: html`<${StageBadge} stage=${contact.stage} t=${t} />` },
		contact.createdAt && { label: "Added", value: formatDate(contact.createdAt) },
	].filter(Boolean);

	return html`<div class="p-4">
		<dl class="grid grid-cols-2 gap-x-6 gap-y-3 max-w-lg">
			${rows.map(
				(row) => html`<div key=${row.label}>
					<dt class="text-xs text-[var(--muted)] mb-0.5">${row.label}</dt>
					<dd class="text-sm text-[var(--text)]">${row.value}</dd>
				</div>`,
			)}
		</dl>
	</div>`;
}

// ── Matters tab ───────────────────────────────────────────────
function MattersTab({ t, onAdd, onEdit, onDelete }) {
	var list = matters.value;

	return html`<div class="p-4">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-sm font-medium text-[var(--text-strong)]">${t("matters.title")}</h3>
			<button class="provider-btn provider-btn-sm" onClick=${onAdd}>${t("matters.add")}</button>
		</div>
		${
			loadingMatters.value
				? html`<p class="text-sm text-[var(--muted)]">Loading\u2026</p>`
				: list.length === 0
					? html`<p class="text-sm text-[var(--muted)]">${t("matters.empty")}</p>`
					: html`<div class="flex flex-col gap-2">
							${list.map(
								(m) => html`<div
									key=${m.id}
									class="rounded-lg border border-[var(--border)] p-3 hover:bg-[var(--bg-hover)] transition-colors"
								>
									<div class="flex items-start justify-between gap-2">
										<div class="flex-1 min-w-0">
											<div class="flex items-center gap-2 flex-wrap">
												<span class="text-sm font-medium text-[var(--text-strong)] truncate">${m.title}</span>
												<span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${MATTER_STATUS_COLORS[m.status] || ""}">${t(`matters.status.${m.status}`)}</span>
											</div>
											<div class="flex items-center gap-2 mt-1 text-xs text-[var(--muted)] flex-wrap">
												<span>${t(`matters.phase.${m.phase}`)}</span>
												<span>·</span>
												<span>${t(`matters.practiceArea.${m.practiceArea}`)}</span>
												${m.createdAt && html`<span>· ${formatDate(m.createdAt)}</span>`}
											</div>
											${m.description && html`<p class="text-xs text-[var(--muted)] mt-1 line-clamp-2">${m.description}</p>`}
										</div>
										<div class="flex gap-1.5 shrink-0">
											<button class="provider-btn provider-btn-sm provider-btn-secondary" onClick=${() => onEdit(m)}>Edit</button>
											<button class="provider-btn provider-btn-sm provider-btn-danger" onClick=${() => onDelete(m)}>Delete</button>
										</div>
									</div>
								</div>`,
							)}
						</div>`
		}
	</div>`;
}

// ── Interactions tab ──────────────────────────────────────────
function InteractionsTab({ t, onAdd, onDelete }) {
	var list = interactions.value;

	return html`<div class="p-4">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-sm font-medium text-[var(--text-strong)]">${t("interactions.title")}</h3>
			<button class="provider-btn provider-btn-sm" onClick=${onAdd}>${t("interactions.add")}</button>
		</div>
		${
			loadingInteractions.value
				? html`<p class="text-sm text-[var(--muted)]">Loading\u2026</p>`
				: list.length === 0
					? html`<p class="text-sm text-[var(--muted)]">${t("interactions.empty")}</p>`
					: html`<div class="flex flex-col gap-2">
							${list.map(
								(i) => html`<div
									key=${i.id}
									class="rounded-lg border border-[var(--border)] p-3"
								>
									<div class="flex items-start justify-between gap-2">
										<div class="flex items-start gap-2 flex-1 min-w-0">
											<span class="icon ${INTERACTION_KIND_ICONS[i.kind] || "icon-file"} text-[var(--muted)] mt-0.5 shrink-0" style="font-size:0.9rem"></span>
											<div class="flex-1 min-w-0">
												<div class="flex items-center gap-2">
													<span class="text-xs font-medium text-[var(--text)]">${t(`interactions.kind.${i.kind}`)}</span>
													${i.createdAt && html`<span class="text-xs text-[var(--muted)]">${formatDateTime(i.createdAt)}</span>`}
												</div>
												<p class="text-sm text-[var(--text)] mt-0.5">${i.summary}</p>
											</div>
										</div>
										<button class="provider-btn provider-btn-sm provider-btn-danger shrink-0" onClick=${() => onDelete(i)}>Delete</button>
									</div>
								</div>`,
							)}
						</div>`
		}
	</div>`;
}

// ── Channels tab ──────────────────────────────────────────────
function ChannelsTab({ t }) {
	var list = channels.value;

	return html`<div class="p-4">
		<h3 class="text-sm font-medium text-[var(--text-strong)] mb-3">${t("detail.channels")}</h3>
		${
			loadingChannels.value
				? html`<p class="text-sm text-[var(--muted)]">Loading\u2026</p>`
				: list.length === 0
					? html`<p class="text-sm text-[var(--muted)]">${t("channels.empty")}</p>`
					: html`<div class="flex flex-col gap-2">
							${list.map(
								(ch) => html`<div
									key=${ch.id}
									class="flex items-center gap-2 rounded-lg border border-[var(--border)] px-3 py-2"
								>
									<span class="text-xs font-mono text-[var(--muted)] bg-[var(--surface2)] px-2 py-0.5 rounded">${ch.channelType}</span>
									<span class="text-sm text-[var(--text)] truncate">${ch.displayName || ch.channelId}</span>
									${ch.verified && html`<span class="text-xs text-green-600 ml-auto">Verified</span>`}
								</div>`,
							)}
						</div>`
		}
	</div>`;
}

// ── Root CRM component ────────────────────────────────────────
function CrmRoot({ contactId }) {
	if (contactId) {
		return html`<${ContactDetailPage} contactId=${contactId} />`;
	}
	return html`<${ContactListPage} />`;
}

// ── Page lifecycle ─────────────────────────────────────────────
var containerRef = null;

export function initCrm(container, param) {
	containerRef = container;
	container.style.cssText = "flex-direction:column;padding:0;overflow:hidden;";

	var contactId = param || null;
	currentContactId.value = contactId;

	if (!contactId) {
		loadContacts();
	}

	render(html`<${CrmRoot} contactId=${contactId} />`, container);
}

export function teardownCrm() {
	if (containerRef) render(null, containerRef);
	containerRef = null;
	// Reset signals
	contacts.value = [];
	loadingContacts.value = true;
	searchQuery.value = "";
	stageFilter.value = "";
	currentContactId.value = null;
	contactDetail.value = null;
	matters.value = [];
	interactions.value = [];
	channels.value = [];
	detailTab.value = "overview";
}

// ── Route registration ────────────────────────────────────────
if (gon.get("crm_enabled")) {
	registerPrefix(routes.crm, initCrm, teardownCrm);
}
