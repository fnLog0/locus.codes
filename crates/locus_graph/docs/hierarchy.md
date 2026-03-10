# LocusGraph — Context ID Hierarchy

Every `context_id` follows strict `{type}:{name}`. Anchors follow `{child_type}_anchor:{parent_name}`.

```
agent:locus
  ├── skill_anchor:locus
  │     ├── skill:rust_debugging
  │     ├── skill:git_best_practices
  │     └── skill:effective_code_review
  ├── learning_anchor:locus
  │     ├── proficiency_anchor:locus
  │     │     ├── proficiency:rust
  │     │     ├── proficiency:typescript
  │     │     └── proficiency:git
  │     ├── preference_anchor:locus
  │     │     ├── preference:response_style
  │     │     ├── preference:commit_format
  │     │     └── preference:naming_convention
  │     ├── pattern_anchor:locus
  │     │     ├── pattern:debug_compile_error
  │     │     ├── pattern:refactor_extract_fn
  │     │     └── pattern:test_before_implement
  │     └── mistake_anchor:locus
  │           ├── mistake:forgot_cargo_check
  │           └── mistake:edited_without_reading
  └── project:locuscodes_abc123
        ├── tool_anchor:locuscodes_abc123
        │     ├── tool:bash
        │     ├── tool:create_file
        │     ├── tool:edit_file
        │     ├── tool:undo_edit
        │     ├── tool:glob
        │     ├── tool:grep
        │     ├── tool:finder
        │     ├── tool:read
        │     ├── tool:task_list
        │     ├── tool:handoff
        │     ├── tool:web_automation
        │     ├── meta:tool_search
        │     ├── meta:tool_explain
        │     ├── meta:task
        │     ├── mcp_anchor:locuscodes_abc123
        │     │     ├── mcp:filesystem-server
        │     │     │     ├── mcp_tool:filesystem-server__read
        │     │     │     └── mcp_tool:filesystem-server__write
        │     │     └── mcp:github-server
        │     │           ├── mcp_tool:github-server__search
        │     │           └── mcp_tool:github-server__pr_create
        │     └── acp_anchor:locuscodes_abc123
        │           └── acp:code-review-agent
        │                 └── acp_tool:code-review-agent__review
        ├── session_anchor:locuscodes_abc123
        │     ├── session:fix-jwt-refresh_a1b2c3d4
        │     │     ├── turn:a1b2c3d4_001
        │     │     │     ├── snapshot:a1b2c3d4_001_001
        │     │     │     ├── intent:a1b2c3d4_001_002
        │     │     │     ├── action:a1b2c3d4_001_003
        │     │     │     ├── action:a1b2c3d4_001_004
        │     │     │     ├── decision:a1b2c3d4_001_005
        │     │     │     └── llm:a1b2c3d4_001_006
        │     │     ├── turn:a1b2c3d4_002
        │     │     │     ├── snapshot:a1b2c3d4_002_001
        │     │     │     ├── intent:a1b2c3d4_002_002
        │     │     │     ├── action:a1b2c3d4_002_003
        │     │     │     ├── file:a1b2c3d4_002_004
        │     │     │     ├── decision:a1b2c3d4_002_005
        │     │     │     ├── llm:a1b2c3d4_002_006
        │     │     │     └── feedback:a1b2c3d4_002_007
        │     │     └── turn:a1b2c3d4_003
        │     │           ├── snapshot:a1b2c3d4_003_001
        │     │           ├── intent:a1b2c3d4_003_002
        │     │           ├── action:a1b2c3d4_003_003
        │     │           ├── error:a1b2c3d4_003_004
        │     │           ├── action:a1b2c3d4_003_005
        │     │           ├── file:a1b2c3d4_003_006
        │     │           ├── action:a1b2c3d4_003_007
        │     │           └── llm:a1b2c3d4_003_008
        │     ├── session:add-mcp-support_e5f6g7h8
        │     │     └── ...
        │     └── session:refactor-ui_i9j0k1l2
        │           └── ...
        ├── skill_anchor:locuscodes_abc123
        │     ├── skill:anyhow_error_pattern
        │     ├── skill:toolbus_api_stable
        │     └── convention:naming_rules
        ├── knowledge_anchor:locuscodes_abc123
        │     ├── fact:rust_error_conventions
        │     ├── fact:project_uses_tokio
        │     └── fact:toolbus_is_safety_layer
        └── learning_anchor:locuscodes_abc123
              ├── proficiency_anchor:locuscodes_abc123
              │     ├── proficiency:locus_toolbus
              │     ├── proficiency:locus_runtime
              │     └── proficiency:locus_graph
              ├── preference_anchor:locuscodes_abc123
              │     ├── preference:error_handling
              │     └── preference:test_style
              ├── pattern_anchor:locuscodes_abc123
              │     ├── pattern:add_new_tool
              │     └── pattern:fix_compile_error
              └── mistake_anchor:locuscodes_abc123
                    ├── mistake:wrong_crate_name
                    └── mistake:missing_re_export
```

## Link Types

### extends — "is a child of" (the tree)

| context_id | extends |
|---|---|
| `agent:locus` | — (root) |
| `skill_anchor:locus` | `agent:locus` |
| `skill:rust_debugging` | `skill_anchor:locus` |
| `project:locuscodes_abc123` | `agent:locus` |
| `tool_anchor:locuscodes_abc123` | `project:locuscodes_abc123` |
| `tool:bash` | `tool_anchor:locuscodes_abc123` |
| `meta:tool_search` | `tool_anchor:locuscodes_abc123` |
| `mcp_anchor:locuscodes_abc123` | `tool_anchor:locuscodes_abc123` |
| `mcp:filesystem-server` | `mcp_anchor:locuscodes_abc123` |
| `mcp_tool:filesystem-server__read` | `mcp:filesystem-server` |
| `acp_anchor:locuscodes_abc123` | `tool_anchor:locuscodes_abc123` |
| `acp:code-review-agent` | `acp_anchor:locuscodes_abc123` |
| `acp_tool:code-review-agent__review` | `acp:code-review-agent` |
| `session_anchor:locuscodes_abc123` | `project:locuscodes_abc123` |
| `session:fix-jwt-refresh_a1b2c3d4` | `session_anchor:locuscodes_abc123` |
| `turn:a1b2c3d4_001` | `session:fix-jwt-refresh_a1b2c3d4` |
| `snapshot:a1b2c3d4_001_001` | `turn:a1b2c3d4_001` |
| `intent:a1b2c3d4_001_002` | `turn:a1b2c3d4_001` |
| `action:a1b2c3d4_001_003` | `turn:a1b2c3d4_001` |
| `decision:a1b2c3d4_001_005` | `turn:a1b2c3d4_001` |
| `error:a1b2c3d4_003_004` | `turn:a1b2c3d4_003` |
| `file:a1b2c3d4_002_004` | `turn:a1b2c3d4_002` |
| `llm:a1b2c3d4_001_006` | `turn:a1b2c3d4_001` |
| `feedback:a1b2c3d4_002_007` | `turn:a1b2c3d4_002` |
| `skill_anchor:locuscodes_abc123` | `project:locuscodes_abc123` |
| `skill:anyhow_error_pattern` | `skill_anchor:locuscodes_abc123` |
| `convention:naming_rules` | `skill_anchor:locuscodes_abc123` |
| `knowledge_anchor:locuscodes_abc123` | `project:locuscodes_abc123` |
| `fact:rust_error_conventions` | `knowledge_anchor:locuscodes_abc123` |
| `learning_anchor:locus` | `agent:locus` |
| `proficiency_anchor:locus` | `learning_anchor:locus` |
| `proficiency:rust` | `proficiency_anchor:locus` |
| `preference_anchor:locus` | `learning_anchor:locus` |
| `preference:response_style` | `preference_anchor:locus` |
| `pattern_anchor:locus` | `learning_anchor:locus` |
| `pattern:debug_compile_error` | `pattern_anchor:locus` |
| `mistake_anchor:locus` | `learning_anchor:locus` |
| `mistake:forgot_cargo_check` | `mistake_anchor:locus` |
| `learning_anchor:locuscodes_abc123` | `project:locuscodes_abc123` |
| `proficiency_anchor:locuscodes_abc123` | `learning_anchor:locuscodes_abc123` |
| `proficiency:locus_toolbus` | `proficiency_anchor:locuscodes_abc123` |
| `preference_anchor:locuscodes_abc123` | `learning_anchor:locuscodes_abc123` |
| `preference:error_handling` | `preference_anchor:locuscodes_abc123` |
| `pattern_anchor:locuscodes_abc123` | `learning_anchor:locuscodes_abc123` |
| `pattern:add_new_tool` | `pattern_anchor:locuscodes_abc123` |
| `mistake_anchor:locuscodes_abc123` | `learning_anchor:locuscodes_abc123` |
| `mistake:wrong_crate_name` | `mistake_anchor:locuscodes_abc123` |

### related_to — "is connected to" (cross-branch associations)

| context_id | related_to | why |
|---|---|---|
| `action:a1b2c3d4_001_003` | `tool:bash` | action used this tool |
| `action:a1b2c3d4_002_003` | `tool:edit_file` | action used this tool |
| `file:a1b2c3d4_002_004` | `skill:anyhow_error_pattern` | edit applied this skill |
| `decision:a1b2c3d4_001_005` | `fact:toolbus_is_safety_layer` | decision based on this knowledge |
| `error:a1b2c3d4_003_004` | `skill:rust_debugging` | error resolved using this skill |
| `skill:anyhow_error_pattern` | `fact:rust_error_conventions` | skill derived from this fact |
| `mcp_tool:filesystem-server__read` | `tool:read` | similar capability |
| `session:fix-jwt-refresh_a1b2c3d4` | `session:add-mcp-support_e5f6g7h8` | related work |
| `proficiency:rust` | `skill:rust_debugging` | proficiency tracks this skill |
| `pattern:add_new_tool` | `skill:toolbus_api_stable` | pattern uses this skill |
| `mistake:wrong_crate_name` | `preference:naming_convention` | mistake violates this preference |

### reinforces — "confirms this is correct" (building confidence)

| context_id | reinforces | why |
|---|---|---|
| `feedback:a1b2c3d4_002_007` | `decision:a1b2c3d4_002_005` | user approved the decision |
| `action:a1b2c3d4_003_007` | `decision:a1b2c3d4_002_005` | tests pass → decision was correct |
| `mcp:filesystem-server` (reconnect) | `mcp:filesystem-server` (original) | same schema, back online |
| `mcp_tool:fs__read` (reconnect) | `mcp_tool:fs__read` (original) | tool confirmed still valid |
| `skill:anyhow_error_pattern` (session 2) | `skill:anyhow_error_pattern` (session 1) | pattern reused successfully |
| `convention:naming_rules` (turn 5) | `convention:naming_rules` (turn 1) | convention followed again |
| `pattern:add_new_tool` (session 3) | `pattern:add_new_tool` (session 1) | workflow reused successfully |
| `feedback:` (approval) | `proficiency:locus_toolbus` | user confirmed agent knows this crate |

### contradicts — "invalidates / corrects" (overriding old beliefs)

| context_id | contradicts | why |
|---|---|---|
| `mcp:filesystem-server` (disconnected) | `mcp:filesystem-server` (connected) | server went offline |
| `mcp_tool:fs__read` (unavailable) | `mcp_tool:fs__read` (available) | tool no longer reachable |
| `mcp_tool:fs__read` (new schema) | `mcp_tool:fs__read` (old schema) | schema changed on reconnect |
| `decision:a1b2c3d4_003_005` | `decision:a1b2c3d4_001_005` | revised approach after error |
| `fact:project_uses_anyhow` | `fact:project_uses_thiserror` | corrected a wrong fact |
| `skill:new_error_pattern` | `skill:old_error_pattern` | learned a better way |
| `preference:error_handling` (updated) | `preference:error_handling` (old) | user corrected the preference |
| `pattern:add_new_tool` (v2) | `pattern:add_new_tool` (v1) | workflow steps changed |
| `mistake:wrong_crate_name` | `proficiency:locus_toolbus` | mistake lowers proficiency |

## Visual — All Four Link Types

```
agent:locus
  └── project:locuscodes_abc123
        ├── tool_anchor:...
        │     └── tool:bash  ←───────────────────────── related_to ── action:a1b2c3d4_001_003
        ├── session_anchor:...                                        (action used this tool)
        │     └── session:fix-jwt_a1b2c3d4
        │           ├── turn:a1b2c3d4_001
        │           │     ├── action:a1b2c3d4_001_003
        │           │     └── decision:a1b2c3d4_001_005 ←─ contradicts ── decision:..._003_005
        │           ├── turn:a1b2c3d4_002                                  (revised after error)
        │           │     ├── decision:a1b2c3d4_002_005 ←─ reinforces ─── feedback:..._002_007
        │           │     └── feedback:a1b2c3d4_002_007                    (user approved)
        │           └── turn:a1b2c3d4_003
        │                 ├── error:a1b2c3d4_003_004 ──── related_to ──→ skill:rust_debugging
        │                 └── decision:a1b2c3d4_003_005                   (resolved using skill)
        ├── skill_anchor:...
        │     └── skill:rust_debugging
        └── knowledge_anchor:...
              └── fact:toolbus_is_safety_layer ←── related_to ── decision:a1b2c3d4_001_005
                                                                 (decision based on this fact)
```

## Graduation Chain

```
mistake → pattern → skill
  "stop doing X"   "when X, do Y"   "I know how to Y"
  (anti-pattern)    (recognized)     (mastered)
```

| Trigger | What happens | Link used |
|---|---|---|
| Same error 3+ times | creates `mistake:` | `related_to` error events |
| Same workflow across 3+ sessions | promotes to `pattern:` | `reinforces` itself |
| `pattern:` used successfully 5+ times | promotes to `skill:` | `extends` skill_anchor |
| `feedback:` approval | bumps `proficiency:` | `reinforces` proficiency |
| `feedback:` rejection | creates/updates `mistake:` | `contradicts` wrong pattern |
| User corrects style | creates/updates `preference:` | `contradicts` old preference |
| `mistake:` not repeated for 10+ sessions | archived (confidence decays) | — |
