# Example: Diff Review

What the Diff Review screen looks like during a patch approval.

---

## Screen

```
┌──────────────────────────────────────────────────────┐
│ Diff Review: Fix authentication bug in login.rs      │
│ Files changed: 2  Additions: +15  Deletions: -3      │
├──────────────────────────────────────────────────────┤
│ Files:                                               │
│  ▸ src/auth/login.rs (+5 -3)                         │
│    src/auth/tests.rs (+10 -0)                        │
├──────────────────────────────────────────────────────┤
│                                                      │
│ src/auth/login.rs                                    │
│                                                      │
│  39 │   /// Validates the given JWT token.            │
│  40 │   /// Returns Ok(()) if valid, Err otherwise.  │
│  41 │   pub fn validate_token(&self, token: &str)    │
│  42 │       -> Result<(), AuthError> {               │
│  43 │-    if token.is_empty() {                      │
│  43 │+    if token.is_empty() {                      │
│  44 │+      return Err(AuthError::EmptyToken);       │
│  45 │+    }                                          │
│  46 │+                                               │
│  47 │+    if self.is_expired(token) {                │
│  48 │       return Err(AuthError::InvalidToken);     │
│  49 │     }                                          │
│  50 │                                                │
│                                                      │
├──────────────────────────────────────────────────────┤
│ [a]pprove  [r]eject  [e]dit  [n]ext file            │
│ > _                                                  │
└──────────────────────────────────────────────────────┘
```

## Flow

1. User reviews the diff
2. Press `n` to see next file (src/auth/tests.rs)
3. Press `a` to approve all changes
4. Pipeline continues: patch applied → tests run

## Rejection Flow

If user presses `r`:
```
> Reason for rejection (optional): Too many separate if blocks, combine into one condition
```
- Rejection sent to Orchestrator
- PatchAgent regenerates with feedback
- New diff shown for review
