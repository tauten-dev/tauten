# kvconf

A layered configuration reader.

```console
$ cat app.conf
# service settings
[server]
port = 8080
hosts = ["a.example", "b.example"]
$ kvconf get app.conf server.port
8080
$ KVCONF_SERVER__PORT=9090 kvconf get --int app.conf server.port
9090
```

Crates:

- `kvconf-kernel`: values, key paths, canonical rendering and the store's
  merge rules (pure, `no_std`);
- `kvconf-core`: the file grammar, loading, typed lookups and `set`; files
  arrive through the `Source` trait and variables through the `Env` trait;
- `kvconf-cli`: the `kvconf` binary; builds the `Source` (rooted at the
  configuration file's directory) and the `Env` (process or `--env-file`).

The full behaviour, including the ground rules every feature follows, is in
[SPEC.md](SPEC.md).

Run the tests with `cargo test`.
