import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";

type ViewMode = "large" | "small" | "list" | "details";

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

const els = {
  content: () => document.querySelector<HTMLElement>("#content")!,
  toasts: () => document.querySelector<HTMLElement>("#toasts")!,
  ctx: () => document.querySelector<HTMLElement>("#context-menu")!,
  ctxRun: () => document.querySelector<HTMLButtonElement>("#ctx-run")!,
  ctxOpen: () => document.querySelector<HTMLButtonElement>("#ctx-open")!,
  ctxDel: () => document.querySelector<HTMLButtonElement>("#ctx-del")!,
  viewButtons: () =>
    Array.from(document.querySelectorAll<HTMLButtonElement>(".seg-btn[data-view]")),
  installBtn: () => document.querySelector<HTMLButtonElement>("#btn-install")!,
  addBtn: () => document.querySelector<HTMLButtonElement>("#btn-add")!,
};

let programs: ProgramEntry[] = [];
let viewMode: ViewMode = "large";
let ctxProgramId: string | null = null;
let selectedIds = new Set<string>();
let primaryAction: "install" | "delete" = "install";

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

function setViewMode(mode: ViewMode) {
  viewMode = mode;
  localStorage.setItem("viewMode", mode);
  document.documentElement.dataset.view = mode;
  for (const btn of els.viewButtons()) {
    btn.classList.toggle("active", btn.dataset.view === mode);
  }
  render();
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
    btn.textContent = count === 0 ? "删除" : `删除 (${count})`;
    btn.title = "删除选中项（同时移除桌面快捷方式和计划任务）";
    return;
  }

  primaryAction = "install";
  const draftsAll = programs.filter((p) => !p.installed).map((p) => p.id);
  const ids = selected.length > 0 ? selectedDraftIds : draftsAll;
  const count = ids.length;
  btn.disabled = count === 0;
  btn.textContent = count === 0 ? "创建" : `创建 (${count})`;
  btn.title = "为草稿创建计划任务和桌面快捷方式";
}

function programIconSrc(p: ProgramEntry) {
  return p.iconDataUrl ?? "";
}

function emptyState() {
  const root = document.createElement("div");
  root.className = "empty";
  root.innerHTML = `
    <div class="empty-title">No items</div>
    <div class="empty-sub">Drop a shortcut (.lnk) to add it to the list.</div>
  `;
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
      const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
      toast("Run failed", msg, "error");
    }
  });
  el.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    showContextMenu(e.clientX, e.clientY, programId);
  });
}

function render() {
  const container = els.content();
  container.replaceChildren();

  if (programs.length === 0) {
    container.appendChild(emptyState());
    updateInstallButton();
    return;
  }

  if (viewMode === "details") {
    const table = document.createElement("table");
    table.className = "details";
    table.innerHTML = `
      <thead>
        <tr><th>Name</th><th>Path</th></tr>
      </thead>
      <tbody></tbody>
    `;
    const tbody = table.querySelector("tbody")!;
    for (const p of programs) {
      const tr = document.createElement("tr");
      tr.dataset.id = p.id;
      if (selectedIds.has(p.id)) tr.classList.add("selected");
      tr.innerHTML = `
        <td class="d-name">
          <img class="d-icon" src="${programIconSrc(p)}" alt="" />
          <span class="d-text"></span>
        </td>
        <td class="d-path"></td>
      `;
      tr.querySelector<HTMLElement>(".d-text")!.textContent = p.installed ? p.name : `${p.name} (draft)`;
      tr.querySelector<HTMLElement>(".d-path")!.textContent = p.targetPath;
      wireItemInteractions(tr, p.id);
      tbody.appendChild(tr);
    }
    container.appendChild(table);
    updateInstallButton();
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
}

async function refreshPrograms() {
  programs = await invoke<ProgramEntry[]>("get_programs");
  // Drop selections that no longer exist
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
      const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
      toast("Add failed", `${p}\n${msg}`, "error");
    }
  }
  await refreshPrograms();
  if (okCount > 0) toast("Added", `${okCount} item(s)`);
}

async function addViaDialog() {
  try {
    const selected = await open({
      multiple: true,
      title: "Add shortcut / executable",
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
    const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
    toast("Open dialog failed", msg, "error");
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
    toast("Created", `${count} shortcut(s) created on Desktop`);
    selectedIds.clear();
    await refreshPrograms();
  } catch (e) {
    const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
    toast("Create failed", msg, "error");
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
      const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
      toast("Delete failed", msg, "error");
    }
  }
  selectedIds.clear();
  await refreshPrograms();
  if (okCount > 0) toast("Deleted", `${okCount} item(s) removed`);
}

async function main() {
  viewMode = getSavedViewMode();
  setViewMode(viewMode);
  await refreshPrograms();

  let suppressNextBlankClick = false;

  // Click empty area to clear selection (Windows Explorer-like).
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

  // Marquee selection (drag to box-select).
  {
    const content = els.content();
    const MIN_DRAG = 4;
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

      const dragged = Math.abs(dx) + Math.abs(dy) >= MIN_DRAG;
      if (!dragged) return;

      updateMarqueeEl({ left, top, width: right - left, height: bottom - top });
      const hits = recomputeHits({ left, top, right, bottom });

      selectedIds = new Set(baseSelection);
      for (const id of hits) selectedIds.add(id);
      updateInstallButton();
      updateSelectionDom();
    });

    const end = (e: PointerEvent) => {
      if (!active || pointerId !== e.pointerId) return;
      active = false;
      pointerId = null;

      const p = clientToContent(e.clientX, e.clientY);
      const dragged = Math.abs(p.x - startX) + Math.abs(p.y - startY) >= MIN_DRAG;
      if (dragged) suppressNextBlankClick = true;

      removeMarqueeEl();
    };

    content.addEventListener("pointerup", end);
    content.addEventListener("pointercancel", end);
  }

  for (const btn of els.viewButtons()) {
    btn.addEventListener("click", () => setViewMode(btn.dataset.view as ViewMode));
  }

  els.addBtn().addEventListener("click", async () => {
    await addViaDialog();
  });

  els.installBtn().addEventListener("click", async () => {
    if (primaryAction === "delete") await deleteSelectedInstalled();
    else await installSelectedOrAllDrafts();
  });

  document.addEventListener("click", (e) => {
    const menu = els.ctx();
    if (!menu.classList.contains("hidden") && !menu.contains(e.target as Node)) {
      hideContextMenu();
    }
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") hideContextMenu();
  });

  els.ctxRun().addEventListener("click", async () => {
    const id = ctxProgramId;
    hideContextMenu();
    if (!id) return;
    try {
      await invoke("run_program", { id });
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
      toast("Run failed", msg, "error");
    }
  });
  els.ctxOpen().addEventListener("click", async () => {
    const id = ctxProgramId;
    hideContextMenu();
    if (!id) return;
    try {
      await invoke("open_program_location", { id });
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
      toast("Open failed", msg, "error");
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
      toast("Deleted", "Desktop shortcut + scheduled task removed");
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as any)?.toString?.() ?? "unknown error";
      toast("Delete failed", msg, "error");
    }
  });

  await getCurrentWindow().onDragDropEvent(async (event) => {
    const dd = event.payload;
    if (dd.type === "drop") {
      const dropped = dd.paths ?? [];
      if (dropped.length > 0) await addDraftsFromDroppedPaths(dropped);
    }
  });
}

window.addEventListener("DOMContentLoaded", () => {
  main().catch((e) => console.error(e));
});
