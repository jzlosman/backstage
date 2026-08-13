export interface PaneLayout {
  projectWidth: number;
  ledgerWidth: number;
  ledgerCollapsed: boolean;
}

export const PROJECT_WIDTH_MIN = 190;
export const PROJECT_WIDTH_MAX = 360;
export const LEDGER_WIDTH_MIN = 280;
export const LEDGER_WIDTH_MAX = 560;

export const DEFAULT_LAYOUT: PaneLayout = {
  projectWidth: 244,
  ledgerWidth: 354,
  ledgerCollapsed: false,
};

const STORAGE_KEY = "backstage.pane-layout.v1";

export function loadPaneLayout(storage: Pick<Storage, "getItem"> = localStorage): PaneLayout {
  try {
    const stored = JSON.parse(storage.getItem(STORAGE_KEY) ?? "null") as Partial<PaneLayout> | null;
    return normalizePaneLayout({ ...DEFAULT_LAYOUT, ...stored });
  } catch {
    return DEFAULT_LAYOUT;
  }
}

export function savePaneLayout(
  layout: PaneLayout,
  storage: Pick<Storage, "setItem"> = localStorage,
): void {
  storage.setItem(STORAGE_KEY, JSON.stringify(normalizePaneLayout(layout)));
}

export function normalizePaneLayout(layout: PaneLayout): PaneLayout {
  return {
    projectWidth: clamp(layout.projectWidth, PROJECT_WIDTH_MIN, PROJECT_WIDTH_MAX),
    ledgerWidth: clamp(layout.ledgerWidth, LEDGER_WIDTH_MIN, LEDGER_WIDTH_MAX),
    ledgerCollapsed: Boolean(layout.ledgerCollapsed),
  };
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, Math.round(value)));
}
