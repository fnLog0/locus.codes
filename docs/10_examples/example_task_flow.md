# Example: Complete Task Flow

**Prompt**: "Fix the authentication bug in login.rs"

---

## Step 1: User Prompt
```
> Fix the authentication bug in login.rs
  [mode: Smart]
```
Prompt sent to Orchestrator.

## Step 2: Orchestrator Builds DAG
```
DAG:
  ├── [parallel]
  │   ├── RepoAgent: scan for login.rs and related auth files
  │   ├── MemoryRecallAgent: recall auth-related memories
  │   └── SearchAgent: grep for "token" and "auth" patterns
  ├── PatchAgent: generate fix (depends on parallel results)
  ├── DiffReview: user approval
  ├── TestAgent: cargo test
  ├── DebugAgent: (conditional, if tests fail)
  └── Commit: (conditional, if user approves)
```

## Step 3: Parallel Subagents Run

**RepoAgent** → ToolBus `glob("**/auth/**")` + `file_read("src/auth/login.rs")`
```
Result: Found src/auth/login.rs, src/auth/mod.rs, src/auth/token.rs
```

**MemoryRecallAgent** → LocusGraph retrieval
```
Recalled 5 locuses:
  0.92 "Auth uses JWT with RS256" (fact:auth_design)
  0.85 "Always validate token expiry" (rule:token_validation)
  0.78 "Previously fixed similar bug by adding expiry check" (fact:auth_fix_history)
```

**SearchAgent** → ToolBus `grep("validate_token", "src/")`
```
Matches: src/auth/login.rs:42, src/auth/token.rs:15
```

## Step 4: Patch Generated

Injection Engine formats memory bundle → injected into PatchAgent's LLM context.

PatchAgent generates:
```diff
--- a/src/auth/login.rs
+++ b/src/auth/login.rs
@@ -42,3 +42,4 @@
   fn validate_token(&self, token: &str) -> Result<(), AuthError> {
-    if token.is_empty() {
+    if token.is_empty() || self.is_expired(token) {
       return Err(AuthError::InvalidToken);
     }
```

## Step 5: Diff Review
User sees the diff in Diff Review view. Presses `a` to approve.

## Step 6: Patch Applied
ToolBus `file_write("src/auth/login.rs", ...)` — applied atomically.

## Step 7: Tests Run
TestAgent → ToolBus `run_cmd("cargo test")`
```
running 12 tests
test auth::test_valid_token ... ok
test auth::test_expired_token ... ok
test auth::test_empty_token ... ok
... 12 passed, 0 failed
```

## Step 8: No Debug Needed
All tests pass. Skip debug loop.

## Step 9: Commit
```
> Commit changes? [y/n] y
git add src/auth/login.rs
git commit -m "fix: validate token expiry in login handler"
  [abc123f]
```

## Step 10: Event Extraction
Events written to LocusGraph:
- `fact:token_expiry_check` — "Added expiry validation to login handler"
- `observation:auth_tests_pass` — "All 12 auth tests pass after fix"
- Reinforces `rule:token_validation` (confidence 0.85 → 0.87)
