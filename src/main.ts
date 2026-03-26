import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";

type ViewMode = "large" | "small" | "list" | "details";
type ThemeMode = "light" | "dark";
type Locale = "zh-CN" | "en";

type ProgramEntry = {
  id: string;
  name: string;
  sourcePath: string;
  targetPath: string;
  arguments?: string | null;
  iconPath: string;
  iconDataUrl?: string | null;
  desktopShortcutPath?: string | null;
  installed: boolean;
  taskName: string;
  createdAtUnixMs: number;
};

type TranslationKey =
  | "add"
  | "create"
  | "delete"
  | "settings"
  | "large"
  | "small"
  | "list"
  | "details"
  | "runElevated"
  | "openLocation"
  | "deleteInstalled"
  | "settingsTitle"
  | "settingsSubtitle"
  | "language"
  | "theme"
  | "themeDark"
  | "themeLight"
  | "emptyTitle"
  | "emptySub"
  | "name"
  | "path"
  | "added"
  | "addFailed"
  | "openDialogFailed"
  | "created"
  | "createFailed"
  | "deleted"
  | "deleteFailed"
  | "runFailed"
  | "openFailed"
  | "statusEmpty"
  | "statusDrafts"
  | "statusSelection"
  | "statusReady";

const translations: Record<Locale, Record<TranslationKey, string>> = {
  "zh-CN": {
    add: "添加",
    create: "创建",
    delete: "删除",
    settings: "设置",
    large: "大图标",
    small: "小图标",
    list: "列表",
    details: "详细信息",
    runElevated: "立即启动",
    openLocation: "打开文件位置",
    deleteInstalled: "删除",
    settingsTitle: "设置",
    settingsSubtitle: "切换语言与浅深主题。",
    language: "语言",
    theme: "主题",
    themeDark: "深色",
    themeLight: "浅色",
    emptyTitle: "暂无项目",
    emptySub: "拖入快捷方式或点击“添加”导入程序。",
    name: "名称",
    path: "路径",
    added: "已添加",
    addFailed: "添加失败",
    openDialogFailed: "打开选择框失败",
    created: "已创建",
    createFailed: "创建失败",
    deleted: "已删除",
    deleteFailed: "删除失败",
    runFailed: "启动失败",
    openFailed: "打开失败",
    statusEmpty: "\u62d6\u5165 .lnk / .exe\uff0c\u6216\u70b9\u51fb\u201c\u6dfb\u52a0\u201d\u5bfc\u5165\u7a0b\u5e8f",
    statusDrafts: "\u5f85\u521b\u5efa\uff1a{count}\u9879\uff0c\u70b9\u51fb\u201c\u521b\u5efa\u201d\u751f\u6210\u684c\u9762\u5feb\u6377\u65b9\u5f0f",
    statusSelection: "\u5df2\u9009\u4e2d {count} \u9879\uff0c\u5f53\u524d\u64cd\u4f5c\uff1a{action}",
    statusReady: "\u5f53\u524d\u89c6\u56fe\uff1a{view}",
  },
  en: {
    add: "Add",
    create: "Create",
    delete: "Delete",
    settings: "Settings",
    large: "Large",
    small: "Small",
    list: "List",
    details: "Details",
    runElevated: "Run elevated",
    openLocation: "Open file location",
    deleteInstalled: "Delete",
    settingsTitle: "Settings",
    settingsSubtitle: "Switch language and light or dark theme.",
    language: "Language",
    theme: "Theme",
    themeDark: "Dark",
    themeLight: "Light",
    emptyTitle: "No items",
    emptySub: "Drop a shortcut here or use Add to import a program.",
    name: "Name",
    path: "Path",
    added: "Added",
    addFailed: "Add failed",
    openDialogFailed: "Open dialog failed",
    created: "Created",
    createFailed: "Create failed",
    deleted: "Deleted",
    deleteFailed: "Delete failed",
    runFailed: "Run failed",
    openFailed: "Open failed",
    statusEmpty: "Drop a .lnk / .exe here, or use Add to import a program",
    statusDrafts: "{count} draft item(s) ready to create",
    statusSelection: "{count} item(s) selected. Current action: {action}",
    statusReady: "Current view: {view}",
  },
};

const els = {
  content: () => document.querySelector<HTMLElement>("#content")!,
  toasts: () => document.querySelector<HTMLElement>("#toasts")!,
  viewBtn: () => document.querySelector<HTMLButtonElement>("#btn-view")!,
  viewLabel: () => document.querySelector<HTMLElement>("#view-label")!,
  viewMenu: () => document.querySelector<HTMLElement>("#view-menu")!,
  viewMenuItems: () => Array.from(document.querySelectorAll<HTMLButtonElement>("#view-menu .menu-item[data-view]")),
  moreBtn: () => document.querySelector<HTMLButtonElement>("#btn-more")!,
  moreMenu: () => document.querySelector<HTMLElement>("#more-menu")!,
  moreSettings: () => document.querySelector<HTMLButtonElement>("#more-settings")!,
  ctx: () => document.querySelector<HTMLElement>("#context-menu")!,
  ctxRun: () => document.querySelector<HTMLButtonElement>("#ctx-run")!,
  ctxOpen: () => document.querySelector<HTMLButtonElement>("#ctx-open")!,
  ctxDel: () => document.querySelector<HTMLButtonElement>("#ctx-del")!,
  installBtn: () => document.querySelector<HTMLButtonElement>("#btn-install")!,
  addBtn: () => document.querySelector<HTMLButtonElement>("#btn-add")!,
  settingsModal: () => document.querySelector<HTMLElement>("#settings-modal")!,
  settingsBackdrop: () => document.querySelector<HTMLElement>("#settings-backdrop")!,
  settingsClose: () => document.querySelector<HTMLButtonElement>("#btn-settings-close")!,
  settingsTitle: () => document.querySelector<HTMLElement>("#settings-title")!,
  settingsSubtitle: () => document.querySelector<HTMLElement>("#settings-subtitle")!,
  languageLabel: () => document.querySelector<HTMLElement>("#settings-language-label")!,
  themeLabel: () => document.querySelector<HTMLElement>("#settings-theme-label")!,
  startupLabel: () => document.querySelector<HTMLElement>("#settings-startup-label")!,
  languageSelect: () => document.querySelector<HTMLSelectElement>("#language-select")!,
  themeSelect: () => document.querySelector<HTMLSelectElement>("#theme-select")!,
  startupToggle: () => document.querySelector<HTMLInputElement>("#startup-toggle")!,
  statusText: () => document.querySelector<HTMLElement>("#status-text")!,
};

let programs: ProgramEntry[] = [];
let viewMode: ViewMode = "large";
let themeMode: ThemeMode = "dark";
let locale: Locale = "zh-CN";
let ctxProgramId: string | null = null;
let selectedIds = new Set<string>();
let primaryAction: "install" | "delete" = "install";
let autostartEnabled: boolean | null = null;

function t(key: TranslationKey) {
  return translations[locale][key];
}

function tr(key: TranslationKey, vars: Record<string, string | number>) {
  let text = t(key);
  for (const [name, value] of Object.entries(vars)) {
    const token = `{${name}}`;
    text = text.split(token).join(String(value));
  }
  return text;
}

function toast(title: string, sub?: string, kind: "info" | "error" = "info") {
  const root = document.createElement("div");
  root.className = `toast ${kind === "error" ? "error" : ""}`.trim();
  root.innerHTML = `
    <div class="toast-title"></div>
    <div class="toast-sub"></div>
  `;
  root.querySelector<HTMLElement>(".toast-title")!.textContent = title;
  root.querySelector<HTMLElement>(".toast-sub")!.textContent = sub ?? "";

  els.toasts().appendChild(root);
  window.setTimeout(() => root.remove(), kind === "error" ? 6500 : 3500);
}

function getSavedViewMode(): ViewMode {
  const raw = localStorage.getItem("viewMode");
  if (raw === "large" || raw === "small" || raw === "list" || raw === "details") return raw;
  return "large";
}

function getSavedTheme(): ThemeMode {
  const raw = localStorage.getItem("themeMode");
  return raw === "light" ? "light" : "dark";
}

function getSavedLocale(): Locale {
  const raw = localStorage.getItem("locale");
  return raw === "en" ? "en" : "zh-CN";
}

function setTheme(mode: ThemeMode) {
  themeMode = mode;
  localStorage.setItem("themeMode", mode);
  document.documentElement.dataset.theme = mode;
  els.themeSelect().value = mode;
}

function applyLocaleText() {
  document.documentElement.lang = locale;
  document.title = "SkipUAC";
  els.addBtn().textContent = t("add");
  els.moreSettings().textContent = t("settings");
  els.ctxRun().textContent = t("runElevated");
  els.ctxOpen().textContent = t("openLocation");
  els.ctxDel().textContent = t("deleteInstalled");
  els.settingsTitle().textContent = t("settingsTitle");
  els.settingsSubtitle().textContent = t("settingsSubtitle");
  els.languageLabel().textContent = t("language");
  els.themeLabel().textContent = t("theme");
  els.startupLabel().textContent =
    locale === "zh-CN" ? "\u5f00\u673a\u81ea\u542f\u52a8" : "Run at startup";

  const themeOptions = els.themeSelect().options;
  themeOptions[0].text = t("themeDark");
  themeOptions[1].text = t("themeLight");

  for (const item of els.viewMenuItems()) {
    const view = item.dataset.view as ViewMode;
    if (view === "large") item.textContent = t("large");
    if (view === "small") item.textContent = t("small");
    if (view === "list") item.textContent = t("list");
    if (view === "details") item.textContent = t("details");
  }
  updateViewLabel();
  updateStatusText();
}

async function refreshAutostartState() {
  try {
    const enabled = await invoke<boolean>("get_autostart_enabled");
    autostartEnabled = enabled;
    els.startupToggle().checked = enabled;
  } catch {
    autostartEnabled = null;
    els.startupToggle().checked = false;
  }
}

function setLocale(next: Locale) {
  locale = next;
  localStorage.setItem("locale", next);
  els.languageSelect().value = next;
  applyLocaleText();
  render();
}

function setViewMode(mode: ViewMode) {
  viewMode = mode;
  localStorage.setItem("viewMode", mode);
  document.documentElement.dataset.view = mode;
  updateViewLabel();
  render();
}

function updateViewLabel() {
  els.viewLabel().textContent = t(viewMode);
  for (const item of els.viewMenuItems()) {
    item.classList.toggle("active", item.dataset.view === viewMode);
  }
}

function hideViewMenu() {
  const menu = els.viewMenu();
  menu.classList.add("hidden");
  menu.setAttribute("aria-hidden", "true");
  els.viewBtn().setAttribute("aria-expanded", "false");
}

function showMenuUnderButton(menu: HTMLElement, btn: HTMLElement) {
  const pad = 8;
  const btnRect = btn.getBoundingClientRect();
  menu.style.left = "0px";
  menu.style.top = "0px";
  const rect = menu.getBoundingClientRect();
  const left = Math.min(window.innerWidth - rect.width - pad, Math.max(pad, btnRect.left));
  const top = Math.min(window.innerHeight - rect.height - pad, Math.max(pad, btnRect.bottom + 8));
  menu.style.left = `${left}px`;
  menu.style.top = `${top}px`;
}

function showViewMenu() {
  const menu = els.viewMenu();
  menu.classList.remove("hidden");
  menu.setAttribute("aria-hidden", "false");
  els.viewBtn().setAttribute("aria-expanded", "true");
  showMenuUnderButton(menu, els.viewBtn());
}

function hideMoreMenu() {
  const menu = els.moreMenu();
  menu.classList.add("hidden");
  menu.setAttribute("aria-hidden", "true");
  els.moreBtn().setAttribute("aria-expanded", "false");
}

function showMoreMenu() {
  const menu = els.moreMenu();
  menu.classList.remove("hidden");
  menu.setAttribute("aria-hidden", "false");
  els.moreBtn().setAttribute("aria-expanded", "true");

  showMenuUnderButton(menu, els.moreBtn());
}

function updateInstallButton() {
  const btn = els.installBtn();
  const selected = programs.filter((p) => selectedIds.has(p.id));
  const selectedDraftIds = selected.filter((p) => !p.installed).map((p) => p.id);
  const selectedInstalledIds = selected.filter((p) => p.installed).map((p) => p.id);

  if (selected.length > 0 && selectedDraftIds.length === 0 && selectedInstalledIds.length > 0) {
    primaryAction = "delete";
    const count = selectedInstalledIds.length;
    btn.disabled = count === 0;
    btn.textContent = count === 0 ? t("delete") : `${t("delete")} (${count})`;
    btn.title = t("deleteInstalled");
    return;
  }

  primaryAction = "install";
  const draftsAll = programs.filter((p) => !p.installed).map((p) => p.id);
  const ids = selected.length > 0 ? selectedDraftIds : draftsAll;
  const count = ids.length;
  btn.disabled = count === 0;
  btn.textContent = count === 0 ? t("create") : `${t("create")} (${count})`;
  btn.title = t("create");
}

function updateStatusText() {
  const drafts = programs.filter((p) => !p.installed).length;
  // Drop any stale selections (e.g., deleted items) so the statusbar doesn't lie.
  {
    const valid = new Set(programs.map((p) => p.id));
    let changed = false;
    for (const id of Array.from(selectedIds)) {
      if (!valid.has(id)) {
        selectedIds.delete(id);
        changed = true;
      }
    }
    if (changed) updateInstallButton();
  }

  const selectedCount = programs.filter((p) => selectedIds.has(p.id)).length;

  if (programs.length === 0) {
    els.statusText().textContent = t("statusEmpty");
    return;
  }

  if (selectedCount > 0) {
    els.statusText().textContent = tr("statusSelection", {
      count: selectedCount,
      action: primaryAction === "delete" ? t("delete") : t("create"),
    });
    return;
  }

  if (drafts > 0) {
    els.statusText().textContent = tr("statusDrafts", { count: drafts });
    return;
  }

  // No selection and nothing pending: keep showing the usage hint instead of a persistent status.
  els.statusText().textContent = t("statusEmpty");
}

function programIconSrc(p: ProgramEntry) {
  return p.iconDataUrl ?? "";
}

function emptyState() {
  const root = document.createElement("div");
  root.className = "empty";
  root.innerHTML = `
    <div class="empty-title"></div>
    <div class="empty-sub"></div>
  `;
  root.querySelector<HTMLElement>(".empty-title")!.textContent = t("emptyTitle");
  root.querySelector<HTMLElement>(".empty-sub")!.textContent = t("emptySub");
  return root;
}

function hideContextMenu() {
  const menu = els.ctx();
  menu.classList.add("hidden");
  menu.style.left = "0px";
  menu.style.top = "0px";
  ctxProgramId = null;
}

function showContextMenu(x: number, y: number, programId: string) {
  ctxProgramId = programId;
  const menu = els.ctx();
  menu.classList.remove("hidden");

  const pad = 8;
  const rect = menu.getBoundingClientRect();
  const maxLeft = window.innerWidth - rect.width - pad;
  const maxTop = window.innerHeight - rect.height - pad;
  menu.style.left = `${Math.max(pad, Math.min(x, maxLeft))}px`;
  menu.style.top = `${Math.max(pad, Math.min(y, maxTop))}px`;
}

function openSettings() {
  els.settingsModal().classList.remove("hidden");
  els.settingsModal().setAttribute("aria-hidden", "false");
  void refreshAutostartState();
}

function closeSettings() {
  els.settingsModal().classList.add("hidden");
  els.settingsModal().setAttribute("aria-hidden", "true");
}

function setSelectionOnly(id: string) {
  selectedIds = new Set([id]);
  updateInstallButton();
  render();
}

function toggleSelection(id: string) {
  if (selectedIds.has(id)) selectedIds.delete(id);
  else selectedIds.add(id);
  updateInstallButton();
  render();
}

function clearSelection() {
  if (selectedIds.size === 0) return;
  selectedIds.clear();
  updateInstallButton();
  render();
}

function updateSelectionDom() {
  const set = selectedIds;
  for (const el of Array.from(els.content().querySelectorAll<HTMLElement>("[data-id]"))) {
    const id = el.dataset.id;
    if (!id) continue;
    el.classList.toggle("selected", set.has(id));
  }
}

function wireItemInteractions(el: HTMLElement, programId: string) {
  el.addEventListener("click", (e) => {
    const multi = (e as MouseEvent).ctrlKey || (e as MouseEvent).metaKey;
    if (multi) toggleSelection(programId);
    else setSelectionOnly(programId);
  });
  el.addEventListener("dblclick", async () => {
    try {
      await invoke("run_program", { id: programId });
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("runFailed"), msg, "error");
    }
  });
  el.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, programId);
  });
}

function render() {
  const container = els.content();
  container.replaceChildren();

  if (programs.length === 0) {
    container.appendChild(emptyState());
    updateInstallButton();
    updateStatusText();
    return;
  }

  if (viewMode === "details") {
    const table = document.createElement("table");
    table.className = "details";
    table.innerHTML = `
      <thead>
        <tr><th>${t("name")}</th><th>${t("path")}</th><th class="d-status-h"></th></tr>
      </thead>
      <tbody></tbody>
    `;
    const tbody = table.querySelector("tbody")!;
    for (const p of programs) {
      const tr = document.createElement("tr");
      tr.dataset.id = p.id;
      if (selectedIds.has(p.id)) tr.classList.add("selected");
      tr.classList.add(p.installed ? "installed" : "draft");
      const statusClass = p.installed ? "installed" : "draft";
      tr.innerHTML = `
        <td class="d-name">
          <img class="d-icon" src="${programIconSrc(p)}" alt="" />
          <span class="d-text"></span>
        </td>
        <td class="d-path"></td>
        <td class="d-status"><span class="status-dot ${statusClass}"></span></td>
      `;
      tr.querySelector<HTMLElement>(".d-text")!.textContent = p.installed ? p.name : `${p.name} (draft)`;
      tr.querySelector<HTMLElement>(".d-path")!.textContent = p.targetPath;
      wireItemInteractions(tr, p.id);
      tbody.appendChild(tr);
    }
    container.appendChild(table);
    updateInstallButton();
    updateStatusText();
    return;
  }

  const list = document.createElement("div");
  if (viewMode === "large") list.className = "grid grid-large";
  else if (viewMode === "small") list.className = "grid grid-small";
  else list.className = "grid grid-rows";

  for (const p of programs) {
    const item = document.createElement("div");
    item.className = "item";
    if (selectedIds.has(p.id)) item.classList.add("selected");
    item.classList.add(p.installed ? "installed" : "draft");
    item.dataset.id = p.id;
    item.innerHTML = `
      <img class="item-icon" src="${programIconSrc(p)}" alt="" />
      <div class="item-text"></div>
    `;
    item.querySelector<HTMLElement>(".item-text")!.textContent = p.installed ? p.name : `${p.name} (draft)`;
    wireItemInteractions(item, p.id);
    list.appendChild(item);
  }

  container.appendChild(list);
  updateInstallButton();
  updateStatusText();
}

async function refreshPrograms() {
  programs = await invoke<ProgramEntry[]>("get_programs");
  const valid = new Set(programs.map((p) => p.id));
  selectedIds = new Set(Array.from(selectedIds).filter((id) => valid.has(id)));
  render();
}

async function addDraftsFromDroppedPaths(paths: string[]) {
  let okCount = 0;
  for (const p of paths) {
    try {
      await invoke("add_program_draft_from_path", { path: p });
      okCount += 1;
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("addFailed"), `${p}\n${msg}`, "error");
    }
  }
  await refreshPrograms();
  if (okCount > 0) toast(t("added"), `${okCount} item(s)`);
}

async function addViaDialog() {
  try {
    const selected = await open({
      multiple: true,
      title: locale === "zh-CN" ? "添加快捷方式 / 可执行文件" : "Add shortcut / executable",
      filters: [
        { name: "Shortcuts / Executables", extensions: ["lnk", "exe"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (selected == null) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) return;
    await addDraftsFromDroppedPaths(paths);
  } catch (e) {
    const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
    toast(t("openDialogFailed"), msg, "error");
  }
}

async function installSelectedOrAllDrafts() {
  const selected = Array.from(selectedIds);
  const ids =
    selected.length > 0
      ? selected.filter((id) => programs.some((p) => p.id === id && !p.installed))
      : programs.filter((p) => !p.installed).map((p) => p.id);
  if (ids.length === 0) return;

  try {
    const count = await invoke<number>("install_programs", { ids });
    toast(t("created"), `${count} shortcut(s) created on Desktop`);
    selectedIds.clear();
    await refreshPrograms();
  } catch (e) {
    const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
    toast(t("createFailed"), msg, "error");
  }
}

async function deleteSelectedInstalled() {
  const ids = Array.from(selectedIds).filter((id) => programs.some((p) => p.id === id && p.installed));
  if (ids.length === 0) return;

  let okCount = 0;
  for (const id of ids) {
    try {
      await invoke("remove_program", { id });
      okCount += 1;
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("deleteFailed"), msg, "error");
    }
  }
  selectedIds.clear();
  await refreshPrograms();
  if (okCount > 0) toast(t("deleted"), `${okCount} item(s) removed`);
}

async function main() {
  locale = getSavedLocale();
  themeMode = getSavedTheme();
  viewMode = getSavedViewMode();

  setTheme(themeMode);
  els.languageSelect().value = locale;
  applyLocaleText();
  setViewMode(viewMode);
  await refreshPrograms();

  let suppressNextBlankClick = false;

  document.addEventListener(
    "contextmenu",
    (e) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest(".item") || target?.closest("tr[data-id]")) return;
      e.preventDefault();
      hideContextMenu();
    },
    true
  );

  els.content().addEventListener("click", (e) => {
    if (suppressNextBlankClick) {
      suppressNextBlankClick = false;
      return;
    }
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (target.closest(".item")) return;
    if (target.closest("tr[data-id]")) return;
    if (target.closest("#context-menu")) return;
    clearSelection();
  });

  {
    const content = els.content();
    const minDrag = 4;
    let active = false;
    let pointerId: number | null = null;
    let startX = 0;
    let startY = 0;
    let baseSelection = new Set<string>();
    let marquee: HTMLDivElement | null = null;

    const clientToContent = (clientX: number, clientY: number) => {
      const rect = content.getBoundingClientRect();
      return {
        x: clientX - rect.left + content.scrollLeft,
        y: clientY - rect.top + content.scrollTop,
      };
    };

    const contentRectForEl = (el: Element) => {
      const rect = (el as HTMLElement).getBoundingClientRect();
      const root = content.getBoundingClientRect();
      return {
        left: rect.left - root.left + content.scrollLeft,
        top: rect.top - root.top + content.scrollTop,
        right: rect.right - root.left + content.scrollLeft,
        bottom: rect.bottom - root.top + content.scrollTop,
      };
    };

    const intersects = (
      a: { left: number; top: number; right: number; bottom: number },
      b: { left: number; top: number; right: number; bottom: number }
    ) => !(a.right < b.left || a.left > b.right || a.bottom < b.top || a.top > b.bottom);

    const updateMarqueeEl = (box: { left: number; top: number; width: number; height: number }) => {
      if (!marquee) {
        marquee = document.createElement("div");
        marquee.className = "marquee";
        content.appendChild(marquee);
      }
      marquee.style.left = `${box.left}px`;
      marquee.style.top = `${box.top}px`;
      marquee.style.width = `${box.width}px`;
      marquee.style.height = `${box.height}px`;
    };

    const removeMarqueeEl = () => {
      marquee?.remove();
      marquee = null;
    };

    const recomputeHits = (selBox: { left: number; top: number; right: number; bottom: number }) => {
      const hit = new Set<string>();
      for (const el of Array.from(content.querySelectorAll<HTMLElement>("[data-id]"))) {
        const id = el.dataset.id;
        if (!id) continue;
        const r = contentRectForEl(el);
        if (intersects(selBox, r)) hit.add(id);
      }
      return hit;
    };

    content.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      const target = e.target as HTMLElement | null;
      if (!target) return;
      if (target.closest(".item")) return;
      if (target.closest("tr[data-id]")) return;
      if (target.closest("#context-menu")) return;
      if (target.closest("button,a,input,textarea,select")) return;

      active = true;
      pointerId = e.pointerId;
      content.setPointerCapture(pointerId);

      const p = clientToContent(e.clientX, e.clientY);
      startX = p.x;
      startY = p.y;

      const multi = e.ctrlKey || e.metaKey;
      baseSelection = multi ? new Set(selectedIds) : new Set<string>();
      if (!multi && selectedIds.size > 0) {
        selectedIds.clear();
        updateInstallButton();
        updateSelectionDom();
        updateStatusText();
      }

      removeMarqueeEl();
      e.preventDefault();
    });

    content.addEventListener("pointermove", (e) => {
      if (!active || pointerId !== e.pointerId) return;
      const p = clientToContent(e.clientX, e.clientY);
      const dx = p.x - startX;
      const dy = p.y - startY;

      const left = Math.min(startX, p.x);
      const top = Math.min(startY, p.y);
      const right = Math.max(startX, p.x);
      const bottom = Math.max(startY, p.y);

      if (Math.abs(dx) + Math.abs(dy) < minDrag) return;

      updateMarqueeEl({ left, top, width: right - left, height: bottom - top });
      const hits = recomputeHits({ left, top, right, bottom });

      selectedIds = new Set(baseSelection);
      for (const id of hits) selectedIds.add(id);
      updateInstallButton();
      updateSelectionDom();
      updateStatusText();
    });

    const end = (e: PointerEvent) => {
      if (!active || pointerId !== e.pointerId) return;
      active = false;
      pointerId = null;

      const p = clientToContent(e.clientX, e.clientY);
      if (Math.abs(p.x - startX) + Math.abs(p.y - startY) >= minDrag) suppressNextBlankClick = true;
      removeMarqueeEl();
    };

    content.addEventListener("pointerup", end);
    content.addEventListener("pointercancel", end);
  }

  els.viewBtn().addEventListener("click", (e) => {
    e.stopPropagation();
    const menu = els.viewMenu();
    if (menu.classList.contains("hidden")) showViewMenu();
    else hideViewMenu();
  });
  for (const item of els.viewMenuItems()) {
    item.addEventListener("click", () => {
      const view = item.dataset.view as ViewMode;
      hideViewMenu();
      setViewMode(view);
    });
  }

  els.addBtn().addEventListener("click", addViaDialog);
  els.installBtn().addEventListener("click", async () => {
    if (primaryAction === "delete") await deleteSelectedInstalled();
    else await installSelectedOrAllDrafts();
  });

  els.moreBtn().addEventListener("click", (e) => {
    e.stopPropagation();
    const menu = els.moreMenu();
    if (menu.classList.contains("hidden")) showMoreMenu();
    else hideMoreMenu();
  });
  els.moreSettings().addEventListener("click", () => {
    hideMoreMenu();
    openSettings();
  });
  els.settingsClose().addEventListener("click", closeSettings);
  els.settingsBackdrop().addEventListener("click", closeSettings);
  els.languageSelect().addEventListener("change", () => setLocale(els.languageSelect().value as Locale));
  els.themeSelect().addEventListener("change", () => setTheme(els.themeSelect().value as ThemeMode));
  els.startupToggle().addEventListener("change", async () => {
    const next = els.startupToggle().checked;
    const prev = autostartEnabled;
    try {
      await invoke("set_autostart_enabled", { enabled: next });
      autostartEnabled = next;
      toast(
        t("settings"),
        next
          ? locale === "zh-CN"
            ? "\u5df2\u5f00\u542f\u5f00\u673a\u81ea\u542f\u52a8"
            : "Enabled run at startup"
          : locale === "zh-CN"
            ? "\u5df2\u5173\u95ed\u5f00\u673a\u81ea\u542f\u52a8"
            : "Disabled run at startup",
      );
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("createFailed"), msg, "error");
      if (prev !== null) els.startupToggle().checked = prev;
      else els.startupToggle().checked = false;
    }
  });

  document.addEventListener("click", (e) => {
    const menu = els.ctx();
    if (!menu.classList.contains("hidden") && !menu.contains(e.target as Node)) {
      hideContextMenu();
    }
    const more = els.moreMenu();
    if (!more.classList.contains("hidden") && !more.contains(e.target as Node) && !els.moreBtn().contains(e.target as Node)) {
      hideMoreMenu();
    }
    const view = els.viewMenu();
    if (!view.classList.contains("hidden") && !view.contains(e.target as Node) && !els.viewBtn().contains(e.target as Node)) {
      hideViewMenu();
    }

    // Clicking outside the content area should also clear selection, otherwise the statusbar can
    // keep showing "selected" even when the user thinks nothing is selected.
    const target = e.target as HTMLElement | null;
    if (!target) return;
    const path = typeof e.composedPath === "function" ? e.composedPath() : [];
    const clickedInContent = path.includes(els.content());
    if (!clickedInContent) {
      if (target.closest("#context-menu")) return;
      if (target.closest("#more-menu")) return;
      if (target.closest("#view-menu")) return;
      if (target.closest("#settings-modal")) return;
      clearSelection();
    }
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      hideContextMenu();
      hideMoreMenu();
      hideViewMenu();
      closeSettings();
    }
  });

  els.ctxRun().addEventListener("click", async () => {
    const id = ctxProgramId;
    hideContextMenu();
    if (!id) return;
    try {
      await invoke("run_program", { id });
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("runFailed"), msg, "error");
    }
  });

  els.ctxOpen().addEventListener("click", async () => {
    const id = ctxProgramId;
    hideContextMenu();
    if (!id) return;
    try {
      await invoke("open_program_location", { id });
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("openFailed"), msg, "error");
    }
  });

  els.ctxDel().addEventListener("click", async () => {
    const id = ctxProgramId;
    hideContextMenu();
    if (!id) return;
    try {
      await invoke("remove_program", { id });
      selectedIds.delete(id);
      await refreshPrograms();
      toast(t("deleted"), locale === "zh-CN" ? "桌面快捷方式和计划任务已移除" : "Desktop shortcut and scheduled task removed");
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { toString?: () => string })?.toString?.() ?? "unknown error";
      toast(t("deleteFailed"), msg, "error");
    }
  });

  await getCurrentWindow().onDragDropEvent(async (event) => {
    const payload = event.payload;
    if (payload.type === "drop") {
      const dropped = payload.paths ?? [];
      if (dropped.length > 0) await addDraftsFromDroppedPaths(dropped);
    }
  });
}

window.addEventListener("DOMContentLoaded", () => {
  main().catch((e) => console.error(e));
});
