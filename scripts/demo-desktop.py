"""在某台 X 显示器上铺一块带大字的桌面，给 VNC 截图用。

用法：
  DISPLAY=:2 DESKTOP_TITLE=$'Cue\\n会话 A' DESKTOP_COLOR=#12382f python3 scripts/demo-desktop.py
"""

import os
import tkinter as tk

title = os.environ.get("DESKTOP_TITLE", "SESSION")
color = os.environ.get("DESKTOP_COLOR", "#12382f")
subtitle = os.environ.get("DESKTOP_SUB", "")

root = tk.Tk()
root.overrideredirect(True)
root.geometry(os.environ.get("DESKTOP_GEOMETRY", "1280x800+0+0"))
root.configure(bg=color)

label = tk.Label(
    root,
    text=title,
    font=("Noto Sans CJK SC", 56),
    bg=color,
    fg="#f4fffb",
    justify="center",
)
label.place(relx=0.5, rely=0.42, anchor="center")

sub = tk.Label(
    root,
    text=subtitle,
    font=("Noto Sans CJK SC", 22),
    bg=color,
    fg="#d5ece4",
    justify="center",
)
sub.place(relx=0.5, rely=0.62, anchor="center")

root.mainloop()
