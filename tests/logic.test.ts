import { describe, expect, it } from "vitest";
import { formatDuration } from "../src/lib/format";
import { matchCombo, matchJump, parseCombo } from "../src/lib/hotkeys";
import { qualityFromLatency } from "../src/lib/quality";
import { cycleTargets, flattenGroups, groupMachines, jumpIndex, type OrderedMachine } from "../src/lib/switchOrder";

function machine(partial: Partial<OrderedMachine> & Pick<OrderedMachine, "id" | "name" | "createdAt">): OrderedMachine {
  return {
    host: "100.64.1.10",
    group: "云电脑",
    kind: "x11vnc",
    tags: [],
    ...partial,
  };
}

function key(partial: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: "",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ...partial,
  } as KeyboardEvent;
}

describe("会话切换顺序", () => {
  const machines = [
    machine({ id: "cue", name: "Cue", createdAt: "2026-01-01T00:00:00Z-00", group: "云电脑" }),
    machine({ id: "box", name: "Instabox", createdAt: "2026-01-01T00:00:00Z-01", group: "云电脑" }),
    machine({ id: "sim", name: "Simular Box", createdAt: "2026-01-01T00:00:00Z-02", group: "云电脑" }),
    machine({ id: "ali", name: "阿里云 Ubuntu", createdAt: "2026-01-01T00:00:00Z-03", group: "云主机" }),
  ];

  it("按添加顺序分组，未分组放最后", () => {
    const extra = machine({ id: "loose", name: "临时", createdAt: "2026-01-01T00:00:00Z-04", group: "" });
    const flat = flattenGroups(groupMachines([...machines, extra], ""));
    expect(flat.map((item) => item.id)).toEqual(["cue", "box", "sim", "ali", "loose"]);
  });

  it("搜索不会打乱未过滤列表的序号语义", () => {
    const filtered = flattenGroups(groupMachines(machines, "ubuntu"));
    expect(filtered.map((item) => item.name)).toEqual(["阿里云 Ubuntu"]);
    expect(jumpIndex(1, machines.length)).toBe(0);
    expect(jumpIndex(4, machines.length)).toBe(3);
    expect(jumpIndex(9, 4)).toBeNull();
  });

  it("Ctrl+Tab 只在已打开的会话之间循环", () => {
    const list = machines.map((item) => ({ id: item.id, live: item.id === "cue" || item.id === "ali" }));
    expect(cycleTargets(list, "cue", 1, true)).toBe("ali");
    expect(cycleTargets(list, "ali", -1, true)).toBe("cue");
    expect(cycleTargets(list, "box", 1, true)).toBe("cue");
  });

  it("一台都没打开时，循环只改变选中项", () => {
    const list = machines.map((item) => ({ id: item.id, live: false }));
    expect(cycleTargets(list, "cue", 1, true)).toBe("box");
    expect(cycleTargets(list, null, -1, false)).toBe("ali");
  });
});

describe("快捷键", () => {
  it("识别 Ctrl+Tab 和数字跳转", () => {
    const next = parseCombo("Ctrl+Tab");
    expect(next).not.toBeNull();
    expect(matchCombo(key({ key: "Tab", ctrlKey: true }), next!)).toBe(true);
    expect(matchCombo(key({ key: "Tab", ctrlKey: true, shiftKey: true }), next!)).toBe(false);
    expect(matchJump(key({ key: "1", ctrlKey: true }), "Ctrl")).toBe(1);
    expect(matchJump(key({ key: "2", ctrlKey: true, shiftKey: true }), "Ctrl")).toBeNull();
    expect(matchJump(key({ key: "4", ctrlKey: true, altKey: true }), "Ctrl+Alt")).toBe(4);
  });
});

describe("质量和时长", () => {
  it("按延迟分档", () => {
    expect(qualityFromLatency(null, null).label).toBe("检测中");
    expect(qualityFromLatency(false, null).level).toBe("offline");
    expect(qualityFromLatency(true, 12).label).toBe("极佳");
    expect(qualityFromLatency(true, 80).label).toBe("良好");
    expect(qualityFromLatency(true, 200).label).toBe("一般");
    expect(qualityFromLatency(true, 800).label).toBe("较差");
  });

  it("格式化连接时长", () => {
    expect(formatDuration(0)).toBe("00:00");
    expect(formatDuration(65_000)).toBe("01:05");
    expect(formatDuration(3_723_000)).toBe("1:02:03");
  });
});
