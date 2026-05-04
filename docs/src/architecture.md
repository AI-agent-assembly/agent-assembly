# Architecture Overview

This chapter describes how `agent-assembly` is composed and how its parts interact at runtime.

## Crate dependency graph

The Cargo workspace contains 16 member crates. Edges in the diagram below are derived from `path` dependencies declared in each crate's `Cargo.toml`.

```mermaid
graph TD
    classDef foundation fill:#e8f1ff,stroke:#5b8def
    classDef ebpf fill:#fdecea,stroke:#d75748
    classDef ffi fill:#eaf6ee,stroke:#3aa55b
    classDef control fill:#fff3d6,stroke:#c98a00
    classDef edge fill:#f3e8ff,stroke:#8b5cf6

    aa_core[aa-core]:::foundation
    aa_proto[aa-proto]:::foundation
    aa_ebpf_common[aa-ebpf-common]:::ebpf

    aa_runtime[aa-runtime]:::foundation
    aa_ebpf[aa-ebpf]:::ebpf
    aa_ebpf_probes[aa-ebpf-probes]:::ebpf
    aa_ebpf_programs[aa-ebpf-programs]:::ebpf
    aa_proxy[aa-proxy]:::ebpf

    aa_gateway[aa-gateway]:::control
    aa_api[aa-api]:::control
    aa_cli[aa-cli]:::control

    aa_ffi_python[aa-ffi-python]:::ffi
    aa_ffi_node[aa-ffi-node]:::ffi
    aa_ffi_go[aa-ffi-go]:::ffi
    aa_wasm[aa-wasm]:::ffi

    conformance[conformance]:::edge

    aa_runtime --> aa_core
    aa_runtime --> aa_proto
    aa_runtime --> aa_ebpf

    aa_ebpf --> aa_core
    aa_ebpf --> aa_ebpf_common
    aa_ebpf_probes --> aa_ebpf_common
    aa_ebpf_programs --> aa_ebpf_common

    aa_proxy --> aa_core
    aa_proxy --> aa_proto
    aa_proxy --> aa_runtime

    aa_ffi_python --> aa_core
    aa_ffi_python --> aa_proto
    aa_ffi_node --> aa_core
    aa_wasm --> aa_core

    aa_gateway --> aa_core
    aa_gateway --> aa_proto
    aa_gateway --> aa_runtime
    aa_api --> aa_core
    aa_api --> aa_gateway
    aa_api --> aa_runtime
    aa_cli --> aa_core
    aa_cli --> aa_gateway

    conformance --> aa_core
    conformance --> aa_proto
```

`aa-ffi-go` has no Cargo dependencies on other workspace crates — it talks to the gateway over gRPC at runtime, with bindings generated from the same `proto/` source as `aa-proto`.
