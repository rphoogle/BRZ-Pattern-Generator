# BRZ Pattern Generator

Angled brick patterns for Brickadia, starting with the honeycomb. Walls at 0°
go on the normal brick grid; walls at any other angle go on entities held at
the right angle by hidden Micro Rigid Bearings, so the whole pattern is one
connected prefab.

**Status: stage 2 of 4** (v0.2.0: the window). Download the .exe from the latest run in the Actions tab (Artifacts).

| Stage | What | State |
|---|---|---|
| 1 | Honeycomb engine, bearings, spokes, area shapes, pillars, self-test | done, tested in game |
| 2 | Window: settings, live preview, counts, presets, save/copy | done, awaiting test |
| 3 | Other patterns, stretch, paint mode | |
| 4 | Borders (clean cut / whole cells, frame), plate | |

## Files
- `src/main.rs` – the window: settings, live preview, counts, presets, save and copy.
- `src/lib.rs` – the pattern engine: works out the pieces, puts each on the
  main grid or a turned entity, finds a hidden spot for each bearing, and
  writes the prefab.
- `src/check.rs` – the self-test: overlapping bricks, bearings in the way,
  sizes, and reading each file back.
- `src/bin/selftest.rs` – runs the self-test over 2,000+ combinations.
- `src/bin/testpack.rs` – writes the in-game test prefabs (T01–T12).

Built with [brdb](https://crates.io/crates/brdb). See the spec
(BRZ_Pattern_Generator_v1_Spec.docx) for the full plan.
