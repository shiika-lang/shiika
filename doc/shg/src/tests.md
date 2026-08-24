# Tests

Directory: `tests/`

- `tests/sk/*.sk`: Shiika-level integration tests. A passing test prints `ok`.
- `tests/erroneous/`: programs that are expected to fail compilation.
- `tests/snapshots/`: snapshot outputs used by the tests above.

For how to run the tests (`rake test`, the `FILTER=` environment variable,
etc.), see the [Development Guide](../../../DEVELOPMENT.md#testing).
