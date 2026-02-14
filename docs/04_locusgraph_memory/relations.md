# Relations

Events can link to other events or context_ids via 4 relation fields on `CreateEventRequest`.

## Relation Fields

### related_to
- **Meaning**: General association
- **Example**: An error event related to a terminal command context

```rust
related_to: Some(vec!["terminal".to_string()])
```

### extends
- **Meaning**: This event refines, updates, or adds detail to another
- **Example**: A more specific fact extending a general project fact

```rust
extends: Some(vec!["project".to_string()])
```

### reinforces
- **Meaning**: This outcome supports another event (positive signal)
- **Example**: Test pass reinforces the approach used in a fix

```rust
reinforces: Some(vec!["editor".to_string()])
```

### contradicts
- **Meaning**: This event overrides or conflicts with another
- **Example**: New decision contradicts a previous decision

```rust
contradicts: Some(vec!["decision:old_approach".to_string()])
```

## How locus.codes Uses Relations

| Scenario | Relation |
|----------|----------|
| Test passes after a fix | `reinforces` → the fix event |
| User rejects a patch | `contradicts` → the patch approach |
| Debug fix builds on first attempt | `extends` → the original fix |
| Error related to a command | `related_to` → `"terminal"` |
| New constraint overrides old rule | `contradicts` → old constraint |

## Effect

Relations help LocusGraph's server-side ranking. Reinforced events surface higher in retrieval. Contradicted events surface lower or are superseded.
