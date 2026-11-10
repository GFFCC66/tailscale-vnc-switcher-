export interface OrderedMachine {
  id: string;
  name: string;
  host: string;
  group: string;
  kind: string;
  tags: string[];
  createdAt: string;
}

export interface MachineGroup<T extends OrderedMachine> {
  name: string;
  items: T[];
}

export interface SwitchTarget {
  id: string;
  live: boolean;
}

/**
 * 侧边栏顺序：分组按组内最早添加的机器排列，「未分组」放最后。
 * 组内保持添加顺序，这样 Ctrl+1 对应眼睛看到的第一台，而不是按名字重排。
 */
export function groupMachines<T extends OrderedMachine>(machines: T[], query: string): MachineGroup<T>[] {
  const needle = query.trim().toLowerCase();
  const filtered = machines.filter((machine) => {
    if (!needle) return true;
    const haystack = [machine.name, machine.host, machine.group, machine.kind, ...machine.tags]
      .join(" ")
      .toLowerCase();
    return haystack.includes(needle);
  });
  const buckets = new Map<string, T[]>();
  for (const machine of filtered) {
    const name = machine.group.trim() || "未分组";
    const list = buckets.get(name) ?? [];
    list.push(machine);
    buckets.set(name, list);
  }
  const groups = [...buckets.entries()].map(([name, items]) => ({
    name,
    items: items.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)),
  }));
  groups.sort((a, b) => {
    if (a.name === "未分组") return 1;
    if (b.name === "未分组") return -1;
    const aStamp = a.items[0]?.createdAt ?? "";
    const bStamp = b.items[0]?.createdAt ?? "";
    return aStamp.localeCompare(bStamp);
  });
  return groups;
}

export function flattenGroups<T extends OrderedMachine>(groups: MachineGroup<T>[]): T[] {
  return groups.flatMap((group) => group.items);
}

export function cycleIndex(current: number, count: number, direction: 1 | -1): number {
  if (count <= 0) return -1;
  if (current < 0) return direction === 1 ? 0 : count - 1;
  return (current + direction + count) % count;
}

export function jumpIndex(digit: number, count: number): number | null {
  if (digit < 1 || digit > 9) return null;
  const index = digit - 1;
  if (index >= count) return null;
  return index;
}

/**
 * Ctrl+Tab 优先在已经打开的会话之间循环。
 * 一台都还没打开时，退回完整列表，只改变选中项，不主动拨号。
 */
export function cycleTargets(
  list: SwitchTarget[],
  activeId: string | null,
  direction: 1 | -1,
  liveOnly: boolean,
): string | null {
  const live = list.filter((item) => item.live);
  const pool = liveOnly && live.length > 0 ? live : list;
  if (pool.length === 0) return null;
  const current = pool.findIndex((item) => item.id === activeId);
  const next = cycleIndex(current, pool.length, direction);
  return pool[next]?.id ?? null;
}
