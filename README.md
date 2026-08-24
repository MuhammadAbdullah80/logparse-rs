# logparse

A streaming access-log summariser in Rust. Point it at a Combined Log Format
file (or pipe one in) and it prints request counts by status class, the slowest
paths by mean latency, and the noisiest clients.

Memory use is proportional to the number of *distinct* paths and hosts, not to
the size of the log, so it handles multi-gigabyte files fine.

## Usage

```sh
cargo run --release -- /var/log/nginx/access.log
cat access.log | cargo run --release
cargo run --release -- --top 25 access.log
```

| Flag | Default | Description |
|------|---------|-------------|
| `--top N` | `10` | Rows printed in each table |

## Example output

```
requests   184203
malformed  12
bytes      1.4 GiB

by status class
  2xx    171204   92.9%
  3xx      4821    2.6%
  4xx      7044    3.8%
  5xx      1134    0.6%

slowest paths (mean)
    842.3ms     311 hits  /api/reports/export
    401.7ms    2210 hits  /api/search
```

## Format

Lines are expected as:

```
host - - [ts] "METHOD /path HTTP/1.1" status bytes "referer" "agent" [duration]
```

The trailing duration is nginx's `$request_time` in seconds and is optional, so
plain Apache CLF lines parse too. Lines that do not match are counted as
malformed rather than aborting the run.

## Design notes

Both tables are bounded by the number of *distinct* paths and hosts, so the
streaming claim holds: a 20GB log costs the same memory as a 20MB one with the
same cardinality.

Paths with fewer than five timed requests are excluded from the slowest-paths
ranking. Without that floor a single 4-second outlier outranks a path serving
fifty thousand requests at 800ms, which is almost never the row you want.

## Tests

```sh
cargo test
```
