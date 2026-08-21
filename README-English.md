# open-fudousan

A land/property search system (Rust, MVP). **The primary purpose is to
smoothly hand off searched site data to the sibling repository
[open-cg-cad](https://github.com/aon-co-jp/open-cg-cad)** (an AI
construction-firm / CAD auto-generation system).

## Important notes (read first)

- **All property data in this repository is dummy data**
  (`crates/fudousan-core/src/dummy_data.rs`, 8 entries). It does not
  represent real real-estate listings.
- No scraping of real real-estate portal sites is performed (out of scope).
- Integration with any real external real-estate API is out of scope.
- Ecosystem-wide development rules are authoritatively defined in
  [`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)'s `CLAUDE.md`
  (see this repository's `CLAUDE.md`).

## Architecture

```
open-fudousan/
├── crates/
│   ├── fudousan-core/   # Data model (Property, etc.) + export conversion for open-cg-cad
│   └── fudousan-api/    # HTTP API implemented directly on tokio/hyper (no web framework)
```

- Following the ecosystem-wide policy of **DB-agnostic, single-binary,
  no template engine** (see `open-raid-z/CLAUDE.md`), `fudousan-api` calls
  `hyper` directly instead of using a web framework such as Poem.
- `fudousan-core::export` converts data into the input format required by
  `open-cg-cad`'s `Site`/`ZoningRules` types (local planar coordinates in
  meters, building coverage ratio, floor area ratio) — see "Integration
  with open-cg-cad" below.

## Build & run

```sh
cargo build --release
cargo test --release
./target/release/fudousan-api   # listens on http://127.0.0.1:8080 by default (override with FUDOUSAN_PORT)
```

## API

- `GET /v1/properties/search?area=<address substring>&budget_max=<yen>` —
  search dummy properties.
- `GET /v1/properties/:id` — single property detail.
- `GET /v1/properties/:id/export-for-cad` — export for open-cg-cad (see below).
- `GET /healthz` — health check.

## Integration with open-cg-cad

`open-cg-cad`'s `crates/cg-cad-core/src/site.rs` requires
`Site { vertices: Vec<(f64, f64)> }` (local planar coordinates, meters),
and `zoning.rs` requires
`ZoningRules { building_coverage_ratio, floor_area_ratio }` (confirmed
directly from open-cg-cad's own source doc comments).

Since `open-fudousan`'s `Property` stores its site shape as latitude/
longitude (`LatLng`), the `export-for-cad` endpoint:

1. Converts the lat/lng vertex list into local planar meters using a
   simple equirectangular approximation anchored at the first vertex
   (`fudousan-core/src/export.rs::to_local_meters`). **Honest limitation**:
   this is not a proper geodetic projection (e.g. UTM); it is adequate for
   small sites (roughly a few hundred meters across) but error grows for
   wide areas.
2. Passes `building_coverage_ratio`/`floor_area_ratio` straight through as
   `CadZoningExport`, matching open-cg-cad's `ZoningRules::new` signature.
3. Always includes `is_dummy_data: true` in the response.

## Verification results

- `cargo build --release` — succeeded.
- `cargo test --release` — 6 unit tests in `fudousan-core`, all passing.
- Real HTTP verification against a running server (via `curl`/
  `Invoke-WebRequest`) — confirmed expected JSON for all 4 endpoints,
  including a 404 for an unknown id.

## What was skipped

- **Stage 4 (persistence to aruaru-db)**: skipped. See the HANDOFF section
  in `CLAUDE.md` for the reason (further investigation of aruaru-db's
  external client connection procedure was judged necessary beyond this
  session's scope).
- `open-cg-cad` currently only implements the site/zoning-check core logic
  (`crates/cg-cad-core`) and has no runnable HTTP integration endpoint, so
  no actual inter-process integration (e.g. calling open-cg-cad over HTTP)
  was performed. This repository is limited to implementing and unit-
  verifying the **data format** open-cg-cad requires.
