# Protocol Versioning Policy

This document defines the versioning scheme, change classification rules, and deprecation lifecycle for the AI Agent Assembly protocol. All changes to proto schemas, JSON schemas, IPC framing, and wire formats are governed by this policy.

---

## Versioning Scheme

The protocol uses **Semantic Versioning (MAJOR.MINOR.PATCH)**:

| Component | Meaning |
|---|---|
| `MAJOR` | Breaking change — existing SDKs must be updated to remain compatible |
| `MINOR` | Non-breaking addition — new fields, new RPCs, new enum values (backward compatible) |
| `PATCH` | Non-breaking fix — documentation corrections, description updates, no wire format change |

The current protocol version is **`protocol/v1`** (pre-stable: `v0.0.1`).

---

## Change Classification

### Non-Breaking Changes (MINOR or PATCH)

These changes can be made without requiring SDK updates:

| Change | Classification | Reason |
|---|---|---|
| Add an optional field to a message | MINOR | Existing decoders ignore unknown fields (proto3) |
| Add a new RPC method to a service | MINOR | Existing clients simply don't call it |
| Add a new enum value | MINOR | Unknown enum values fall back to `_UNSPECIFIED = 0` |
| Add a new service | MINOR | Existing clients don't depend on it |
| Rename a field **description** (not the field itself) | PATCH | No wire format change |
| Fix a typo in a comment or doc string | PATCH | No wire format change |
| Tighten a JSON Schema description | PATCH | No wire format change |

### Breaking Changes (MAJOR)

These changes require a MAJOR version bump and a migration guide:

| Change | Classification | Reason |
|---|---|---|
| Remove a field from a message | MAJOR | Existing encoders/decoders break |
| Rename a field | MAJOR | Field number stays but name change breaks JSON/gRPC-gateway |
| Change a field's type | MAJOR | Wire encoding changes |
| Change a field number | MAJOR | Proto3 wire encoding is field-number based |
| Remove an RPC method | MAJOR | Existing callers get `UNIMPLEMENTED` errors |
| Remove an enum value | MAJOR | Existing code holding that value breaks |
| Add a required field | MAJOR | Existing messages missing the field become invalid |
| Change a JSON Schema `type` constraint | MAJOR | Existing valid documents become invalid |
| Narrow a JSON Schema constraint (e.g. add `minLength`) | MAJOR | Previously valid values may now fail validation |
