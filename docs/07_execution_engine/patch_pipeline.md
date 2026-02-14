# Patch Pipeline

How code changes are generated and applied.

## Flow

```
Task analysis → File identification (RepoAgent)
  → Context assembly (memories + files + search)
  → Patch generation (PatchAgent via LLM)
  → Diff creation (unified format)
  → Validation (syntax check, constraint check)
  → Diff Review (user approval)
  → Apply atomically
  → Rollback on failure
```

## Patch Format

Patches are **unified diffs**:

```diff
--- a/src/auth/login.rs
+++ b/src/auth/login.rs
@@ -42,3 +42,4 @@
   fn validate_token(&self) {
-    if token.is_empty() {
+    if token.is_empty() || token.is_expired() {
       return Err(AuthError::InvalidToken);
     }
```

## Validation

Before showing to user:
1. Patch applies cleanly (no conflicts)
2. ConstraintAgent checks against active constraints
3. Basic syntax validation (file parses after patch)

## Application

- Patches applied **atomically** — all or nothing
- Working directory state saved before apply
- On failure → automatic rollback to saved state
- Applied via ToolBus `file_write` (permission-checked)

## Multi-file Patches

When a task requires changes to multiple files:
- Each file gets its own diff hunk
- All shown together in Diff Review
- Applied atomically as a group
