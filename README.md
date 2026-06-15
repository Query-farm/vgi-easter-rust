# vgi-easter (Rust)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A tiny [VGI (Vector Gateway Interface)](https://github.com/Query-farm) worker
that gives DuckDB one SQL function — `easter_date(year)` — returning the date of
Western (Gregorian) Easter Sunday. It has no external data and almost no code,
which makes it a clean, copyable example of a VGI scalar-function worker.

This is a Rust port of the [Python `vgi-easter`](https://pypi.org/project/vgi-easter/)
worker, built on the [`vgi`](https://crates.io/crates/vgi) crate.

## Quick start

Build the worker, then attach it from DuckDB over stdio:

```bash
cargo build --release
```

In DuckDB:

```sql
-- VGI isn't bundled with DuckDB yet, so load it from the community channel
-- (INSTALL is a one-time download; LOAD is once per session).
INSTALL vgi FROM community;
LOAD vgi;

ATTACH 'easter' (TYPE 'vgi', LOCATION '/path/to/target/release/vgi-easter');

SELECT easter.main.easter_date(2025);   -- 2025-04-20

SELECT year, easter.main.easter_date(year) AS easter
FROM range(2020, 2025) t(year);
```

DuckDB launches the worker for you, and `easter_date` then behaves like a native
function (a null year yields a null date).

## How it works

A VGI worker publishes catalogs, schemas, and functions that DuckDB can `ATTACH`
and query as if they were built in. Values cross the boundary as Apache Arrow,
so they stay columnar end to end.

This worker publishes a single function:

```
easter                                  (catalog)
└── main                                (schema)
    └── easter_date(year BIGINT) → DATE
```

The implementation is in [`src/easter.rs`](src/easter.rs) (the date calculation:
`easter_sunday`, the Anonymous Gregorian *Computus*, plus the civil-date →
`date32` conversion) and [`src/main.rs`](src/main.rs) (the `EasterDateFunction`
`ScalarFunction` mapping an Arrow `Int64Array` of years to a `Date32Array`, and
the few lines wiring it into a catalog).

## Running it

| Invocation              | Transport | Use it when                                               |
| ----------------------- | --------- | --------------------------------------------------------- |
| `vgi-easter`            | stdio     | DuckDB spawns the worker as a subprocess (the quickstart) |
| `vgi-easter --http`     | HTTP      | you want a long-running server to attach to               |
| `vgi-easter --unix <p>` | AF_UNIX   | the VGI launcher manages the socket                       |

The HTTP transport binds an ephemeral port on `127.0.0.1` and announces it as
`PORT:<n>` on stdout. The provided [`Dockerfile`](Dockerfile) /
[`docker-entrypoint.sh`](docker-entrypoint.sh) bridge that to `0.0.0.0:8080`
with `socat` for container/Fly deployment:

```sql
LOAD vgi;   -- after a one-time INSTALL vgi FROM community
ATTACH 'easter' (TYPE 'vgi', LOCATION 'https://vgi-easter-rust.fly.dev');
```

## Configuration

| Variable                  | Purpose                                                        |
| ------------------------- | ------------------------------------------------------------- |
| `VGI_EASTER_GIT_COMMIT`   | Reported as the catalog's `implementation_version` (else the crate version). |
| `VGI_LOG`                 | Log level for the worker (default `info`).                     |
| `PORT`                    | Public port the Docker entrypoint bridges to (default `8080`). |

## Development

Requires Rust 1.86+.

```bash
cargo test
```

The unit tests cover the Easter calculation (including the March 22 / April 25
extremes), the `date32` conversion, the Arrow compute path (with null
propagation), and version resolution. A separate
[sqllogictest](https://duckdb.org/dev/sqllogictest/intro) suite in `test/sql/`
drives the worker through the **real** DuckDB `vgi` extension. CI runs both on
Linux, macOS, and Windows — see [`ci/README.md`](ci/README.md).

## Built with

- **[DuckDB](https://duckdb.org)** attaches and queries the worker — install the
  VGI extension with `INSTALL vgi FROM community; LOAD vgi;`.
- **[vgi](https://crates.io/crates/vgi)** is the VGI worker SDK this is built on.
- **[Haybarn](https://github.com/Query-farm-haybarn)** provides the
  DuckDB-compatible `unittest` runner that drives the cross-platform CI.

## License

MIT — see [LICENSE](LICENSE). Copyright 2026 Query Farm LLC — https://query.farm
