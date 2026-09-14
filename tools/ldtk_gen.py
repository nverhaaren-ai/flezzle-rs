#!/usr/bin/env python3
"""Generate flezzle-rs level files (LDtk 1.5.3 JSON) from ASCII maps.

Two subcommands:

  retarget SRC DEST     Copy an existing LDtk project, rewriting tileset paths
                        for the assets/levels/ layout (tilesets live one
                        directory up, in assets/atlas/).
  gen MAP.txt DEST      Build a single-level project from an ASCII map, using
                        TEMPLATE for all definitions (layers, entities,
                        tilesets, enums).

ASCII map legend (one character per 16px cell):
  .  empty          #  dirt (IntGrid 1)     =  stone (IntGrid 3)
  H  ladder (2)     P  player spawn         C  chest
  M  mob            m  mob patrol target (nearest on the same row)
  p  pumpkins (decorative)

Lines starting with ';' are comments. All map rows are padded to the widest.

Visuals: LDtk normally bakes auto-layer tiles into the file when you save in
the editor. We can't run the editor here, so the Collisions layer's tiles are
chosen from a lookup table *learned* from the template's baked levels, keyed
by each cell's 3x3 IntGrid neighbourhood. The decorative layers stay empty.
Open the result in LDtk and save to get the real auto-tiling.
"""
import argparse
import copy
import json
import random
import sys
import uuid
from collections import Counter, defaultdict
from pathlib import Path

GRID = 16
SOLID = {"#": 1, "H": 2, "=": 3}
ENTITIES = {"P": "Player", "C": "Chest", "M": "Mob", "p": "Pumpkins"}
ATLAS_PREFIX = "../atlas/"


def load(path):
    with open(path) as f:
        return json.load(f)


def dump(obj, path):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        if obj.get("minifyJson"):
            json.dump(obj, f, separators=(",", ":"))
        else:
            json.dump(obj, f, indent="\t")
        f.write("\n")


def retarget_paths(project):
    """Point tileset paths at ../atlas/ (levels live in assets/levels/)."""
    for ts in project["defs"]["tilesets"]:
        rel = ts.get("relPath")
        if rel and not rel.startswith(ATLAS_PREFIX):
            ts["relPath"] = ATLAS_PREFIX + rel.split("/")[-1]
    for level in project["levels"]:
        for li in level["layerInstances"]:
            rel = li.get("__tilesetRelPath")
            if rel and not rel.startswith(ATLAS_PREFIX):
                li["__tilesetRelPath"] = ATLAS_PREFIX + rel.split("/")[-1]


# ---------------------------------------------------------------- learning

def neighbourhood(grid, w, h, x, y):
    """3x3 IntGrid values around (x, y); out of bounds counts as empty."""
    out = []
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            nx, ny = x + dx, y + dy
            out.append(grid[ny * w + nx] if 0 <= nx < w and 0 <= ny < h else 0)
    return tuple(out)


def learn_tile_table(project, layer_identifier="Collisions"):
    """Map 3x3 neighbourhood -> most common baked tile stack in the template."""
    stacks = defaultdict(Counter)   # key -> Counter of tile-stack tuples
    for level in project["levels"]:
        for li in level["layerInstances"]:
            if li["__identifier"] != layer_identifier:
                continue
            w, h, grid = li["__cWid"], li["__cHei"], li["intGridCsv"]
            per_cell = defaultdict(list)
            for t in li["autoLayerTiles"]:
                cx, cy = t["px"][0] // GRID, t["px"][1] // GRID
                per_cell[(cx, cy)].append((t["t"], tuple(t["src"]), t["f"], t["d"][0]))
            for y in range(h):
                for x in range(w):
                    key = neighbourhood(grid, w, h, x, y)
                    stack = tuple(per_cell.get((x, y), []))
                    stacks[key][stack] += 1
                    # coarser fallbacks: 4-neighbourhood, then value only
                    v = grid[y * w + x]
                    n4 = (v, key[1], key[7], key[3], key[5])
                    stacks[("n4",) + n4][stack] += 1
                    stacks[("v", v)][stack] += 1
    return {k: c.most_common(1)[0][0] for k, c in stacks.items()}


def tiles_for(table, grid, w, h, x, y):
    key = neighbourhood(grid, w, h, x, y)
    v = grid[y * w + x]
    for k in (key, ("n4", v, key[1], key[7], key[3], key[5]), ("v", v)):
        if k in table:
            return table[k]
    return ()


# --------------------------------------------------------------- generation

def parse_map(text):
    rows = [ln.rstrip("\n") for ln in text.splitlines() if not ln.startswith(";")]
    while rows and not rows[-1].strip():
        rows.pop()
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".") for r in rows]


def field_instances(entity_def, patrol_points):
    fields = []
    for fd in entity_def["fieldDefs"]:
        fi = {
            "__identifier": fd["identifier"],
            "__type": fd["__type"],
            "__value": [] if fd["isArray"] else None,
            "__tile": None,
            "defUid": fd["uid"],
            "realEditorValues": [],
        }
        if fd["identifier"] == "patrol":
            fi["__value"] = [{"cx": cx, "cy": cy} for cx, cy in patrol_points]
            fi["realEditorValues"] = [
                {"id": "V_String", "params": [f"{cx},{cy}"]} for cx, cy in patrol_points
            ]
        elif fd["__type"] == "Bool":
            fi["__value"] = False
        fields.append(fi)
    return fields


def make_entity(entity_def, cx, cy, patrol_points=()):
    w, h = entity_def["width"], entity_def["height"]
    px = [cx * GRID + GRID // 2, (cy + 1) * GRID]  # pivot (0.5, 1): bottom-centre
    tile = entity_def.get("tileRect")
    return {
        "__identifier": entity_def["identifier"],
        "__grid": [cx, cy],
        "__pivot": [entity_def["pivotX"], entity_def["pivotY"]],
        "__tags": list(entity_def.get("tags", [])),
        "__tile": dict(tile) if tile else None,
        "__smartColor": entity_def["color"],
        "iid": str(uuid.uuid4()),
        "width": w,
        "height": h,
        "defUid": entity_def["uid"],
        "px": px,
        "__worldX": px[0],
        "__worldY": px[1],
        "fieldInstances": field_instances(entity_def, patrol_points),
    }


def generate(template, map_rows, identifier, rng):
    defs = template["defs"]
    entity_defs = {e["identifier"]: e for e in defs["entities"]}
    h, w = len(map_rows), len(map_rows[0])
    grid = [0] * (w * h)
    entities = []
    for y, row in enumerate(map_rows):
        for x, ch in enumerate(row):
            if ch in SOLID:
                grid[y * w + x] = SOLID[ch]
    for y, row in enumerate(map_rows):
        for x, ch in enumerate(row):
            if ch not in ENTITIES:
                continue
            patrol = []
            if ch == "M":
                targets = [i for i, c in enumerate(row) if c == "m"]
                if targets:
                    patrol = [(min(targets, key=lambda i: abs(i - x)), y)]
            entities.append(make_entity(entity_defs[ENTITIES[ch]], x, y, patrol))

    table = learn_tile_table(template)
    skeleton_level = template["levels"][0]
    level = copy.deepcopy(skeleton_level)
    level.update({
        "identifier": identifier,
        "iid": str(uuid.uuid4()),
        "uid": 0,
        "worldX": 0, "worldY": 0, "worldDepth": 0,
        "pxWid": w * GRID, "pxHei": h * GRID,
        "fieldInstances": [],
        "__neighbours": [],
    })
    layer_instances = []
    for skel in skeleton_level["layerInstances"]:
        li = copy.deepcopy(skel)
        li.update({
            "__cWid": w, "__cHei": h,
            "iid": str(uuid.uuid4()),
            "levelId": 0,
            "seed": rng.randrange(1, 10_000_000),
            "intGridCsv": [],
            "autoLayerTiles": [],
            "gridTiles": [],
            "entityInstances": [],
        })
        if li["__identifier"] == "Entities":
            li["entityInstances"] = entities
        elif li["__identifier"] == "Collisions":
            li["intGridCsv"] = grid
            tiles = []
            for y in range(h):
                for x in range(w):
                    for (t, src, f, rule) in tiles_for(table, grid, w, h, x, y):
                        tiles.append({
                            "px": [x * GRID, y * GRID], "src": list(src),
                            "f": f, "t": t, "d": [rule, y * w + x], "a": 1,
                        })
            li["autoLayerTiles"] = tiles
        layer_instances.append(li)
    level["layerInstances"] = layer_instances

    project = copy.deepcopy(template)
    project["iid"] = str(uuid.uuid4())
    project["levels"] = [level]
    project["toc"] = []
    project["nextUid"] = max(project["nextUid"], 1)
    return project


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("retarget"); r.add_argument("src"); r.add_argument("dest")
    g = sub.add_parser("gen"); g.add_argument("map"); g.add_argument("dest")
    g.add_argument("--template", default="assets/levels/example_world.ldtk")
    g.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()

    if args.cmd == "retarget":
        project = load(args.src)
        retarget_paths(project)
        dump(project, args.dest)
    elif args.cmd == "gen":
        template = load(args.template)
        rows = parse_map(Path(args.map).read_text())
        identifier = Path(args.map).stem.replace("-", "_").capitalize()
        dump(generate(template, rows, identifier, random.Random(args.seed)), args.dest)
    print(f"wrote {args.dest}")


if __name__ == "__main__":
    sys.exit(main())
