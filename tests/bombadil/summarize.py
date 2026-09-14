#!/usr/bin/env python3
"""Summarize Bombadil runs: violations per property and game-snapshot stats.

Usage: tests/bombadil/summarize.py [target/bombadil] [--json]
Reads <dir>/<level>/trace.jsonl for each level directory.
"""
import json
import sys
from collections import Counter
from pathlib import Path


def cell(row, name):
    for s in row.get("snapshots", []):
        if s.get("name") == name:
            return s.get("value")
    return None


def summarize(trace_path: Path) -> dict:
    rows = [json.loads(line) for line in trace_path.open()]
    violations = Counter()
    for row in rows:
        for v in row.get("violations") or []:
            violations[v.get("name", "?")] += 1
    actions = Counter()
    for row in rows:
        a = row.get("action")
        if not a:
            continue
        if "Custom" in a:
            actions[f"Custom:{a['Custom']['name']}"] += 1
        else:
            actions[next(iter(a))] += 1
    snaps = [cell(r, "snap") for r in rows]
    snaps = [s for s in snaps if s]
    stats = {}
    if snaps:
        last = snaps[-1]
        stats = {
            "states": len(rows),
            "states_with_snapshot": len(snaps),
            "frames": last.get("frame"),
            "ticks": last.get("tick"),
            "zero_tick_frames": last.get("zero_tick_frames"),
            "multi_tick_frames": last.get("multi_tick_frames"),
            "max_ticks_per_frame_seen": max(s.get("ticks_this_frame", 0) for s in snaps),
            "max_walk_speed_error": max(s.get("max_walk_speed_error", 0) or 0 for s in snaps),
            "max_walk_step": max(s.get("max_walk_step", 0) or 0 for s in snaps),
            "level_spawns": last.get("level_spawns"),
            "level_despawns": last.get("level_despawns"),
            "player_z": (last.get("player") or {}).get("z"),
            "chest_z": (last.get("chest") or {}).get("z"),
            "chest_x_range": None,
            "player_x_range": None,
            "push_scenario_states": 0,
        }
        cx = [s["chest"]["pos"]["x"] for s in snaps if s.get("chest")]
        px = [s["player"]["pos"]["x"] for s in snaps if s.get("player")]
        if cx:
            stats["chest_x_range"] = [round(min(cx), 1), round(max(cx), 1)]
        if px:
            stats["player_x_range"] = [round(min(px), 1), round(max(px), 1)]
        for s in snaps:
            p, c = s.get("player"), s.get("chest")
            if p and c and abs(p["pos"]["y"] - c["pos"]["y"]) <= 12:
                dx = c["pos"]["x"] - p["pos"]["x"]
                if 0 < dx < 26 and (s.get("held_actions", 0) & 2) and p.get("on_ground"):
                    stats["push_scenario_states"] += 1
    return {"violations": dict(violations), "actions": dict(actions), "stats": stats}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    root = Path(args[0] if args else "target/bombadil")
    out = {}
    for trace in sorted(root.glob("*/trace.jsonl")):
        out[trace.parent.name] = summarize(trace)
    if "--json" in sys.argv:
        print(json.dumps(out, indent=2))
        return
    for level, r in out.items():
        print(f"=== {level}")
        print("  violations:", r["violations"] or "none")
        print("  actions:", r["actions"])
        for k, v in r["stats"].items():
            print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
