# Test Runner

TestAgent runs project tests after patch application.

## Auto-Detection

The test framework is auto-detected from project files:

| Indicator | Framework | Command |
|-----------|-----------|---------|
| `Cargo.toml` | cargo test | `cargo test` |
| `package.json` (scripts.test) | npm/yarn | `npm test` |
| `pytest.ini` / `conftest.py` | pytest | `pytest` |
| `go.mod` | go test | `go test ./...` |
| `Makefile` (test target) | make | `make test` |

## Execution

1. TestAgent determines test command from Session Manager metadata
2. Runs via ToolBus `run_cmd` (sandboxed)
3. Captures stdout, stderr, exit code
4. Parses output for pass/fail per test

## Reporting

```
TestResult {
    total: 12,
    passed: 11,
    failed: 1,
    skipped: 0,
    failures: [
        { name: "test_token_expiry", output: "assertion failed..." }
    ],
    duration: 3.2s
}
```

## On Failure

- TestAgent reports failure to Orchestrator
- Orchestrator activates DebugAgent loop
- Failure output passed as context to DebugAgent

## Events

Test results stored as observations in LocusGraph:
- Pass → reinforces related locuses
- Fail → stored as learning (what didn't work)
