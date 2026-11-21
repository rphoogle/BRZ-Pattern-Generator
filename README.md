# BRZ Pattern Generator

Angled brick patterns for Brickadia: honeycomb, triangles, star of David,
cube illusion, octagons and squares, and diamonds. Walls at 0°
go on the normal brick grid; walls at any other angle go on entities held at
the right angle by hidden Micro Rigid Bearings, so the whole pattern is one
connected prefab.

**Status: stage 3 of 4** (v0.3.1: palettes, hidden bearings, thick walls, side view). Download the .exe from the latest run in the Actions tab (Artifacts).

| Stage | What | State |
|---|---|---|
| 1 | Honeycomb engine, bearings, spokes, area shapes, pillars, self-test | done, tested in game |
| 2 | Window: settings, live preview, counts, presets, save/copy | done, tested in game (v0.2.1) |
| 3a | Five other patterns (v0.3.0) | done |
| 3b | Palettes, hidden bearings, main walls on an entity, no thickness cap, side view (v0.3.1) | done, awaiting test |
| 3c | Stretch, paint mode, PNG mask (v0.3.2) | |
| 4 | Borders (clean cut / whole cells, frame), plate | |

## Files
- `src/main.rs` – the window: settings, live preview, counts, presets, save and copy.
- `src/lib.rs` – the pattern engine: works out the pieces, puts each on the
  main grid or a turned entity, finds a hidden spot for each bearing, and
  writes the prefab.
- `src/patterns.rs` – the other five patterns: draws each as tiles, turns
  every tile edge into a wall, groups walls by direction onto grids, and joins
  walls that carry straight on into one long brick.
- `src/layers.rs` – final touches: which colour each entity uses, main walls onto
  their own entity, and moving overlapping thick walls onto extra entities.
- `src/palette.rs` – reads and writes the game's colour palette files (.bp),
  converting the stored linear colours to on-screen colours. Shared with the suite.
- `src/check.rs` – the self-test: overlapping bricks, bearings in the way,
  sizes, and reading each file back.
- `src/bin/selftest.rs` – runs the self-test over 12,000+ combinations.
- `src/bin/testpack.rs` – writes the in-game test prefabs for this version, with a top-down picture of each.
- `src/bin/previewpng.rs` – draws every pattern from above (for checking shapes).

Built with [brdb](https://crates.io/crates/brdb). See the spec
(BRZ_Pattern_Generator_v1_Spec.docx) for the full plan.
