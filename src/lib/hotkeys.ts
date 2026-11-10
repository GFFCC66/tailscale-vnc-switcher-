export interface Combo {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
  key: string;
}

const MODIFIERS = new Set(["ctrl", "control", "shift", "alt", "option", "meta", "cmd", "super"]);

export function parseCombo(input: string): Combo | null {
  const parts = input
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) return null;
  const combo: Combo = { ctrl: false, shift: false, alt: false, meta: false, key: "" };
  for (const part of parts) {
    const lowered = part.toLowerCase();
    if (lowered === "ctrl" || lowered === "control") combo.ctrl = true;
    else if (lowered === "shift") combo.shift = true;
    else if (lowered === "alt" || lowered === "option") combo.alt = true;
    else if (lowered === "meta" || lowered === "cmd" || lowered === "super") combo.meta = true;
    else combo.key = normalizeKey(part);
  }
  if (!combo.key) return null;
  return combo;
}

export function matchCombo(event: KeyboardEvent, combo: Combo): boolean {
  if (event.ctrlKey !== combo.ctrl) return false;
  if (event.shiftKey !== combo.shift) return false;
  if (event.altKey !== combo.alt) return false;
  if (event.metaKey !== combo.meta) return false;
  return normalizeKey(event.key) === combo.key;
}

/** Ctrl / Ctrl+Alt 这类修饰键加上数字 1–9。 */
export function matchJump(event: KeyboardEvent, modifier: string): number | null {
  if (!/^[1-9]$/.test(event.key)) return null;
  const combo = parseCombo(`${modifier}+${event.key}`);
  if (!combo || !matchCombo(event, combo)) return null;
  return Number(event.key);
}

export function comboFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIERS.has(event.key.toLowerCase())) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Meta");
  parts.push(displayKey(event.key));
  if (parts.length === 1 && event.key !== "F11" && event.key.length === 1) return null;
  return parts.join("+");
}

function displayKey(key: string): string {
  if (key.length === 1) return key.toUpperCase();
  if (/^f\d+$/i.test(key)) return key.toUpperCase();
  if (key === "Tab") return "Tab";
  if (key === "Escape") return "Escape";
  if (key === "Enter") return "Enter";
  return key;
}

export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

function normalizeKey(key: string): string {
  const lowered = key.toLowerCase();
  if (lowered === "esc") return "escape";
  if (lowered === "tab") return "tab";
  if (lowered === "f11") return "f11";
  if (key.length === 1) return lowered;
  return lowered;
}
