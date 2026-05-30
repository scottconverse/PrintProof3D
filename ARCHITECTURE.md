# PrintProof3D System Architecture & Drawings

This document describes the design principles, crate boundaries, data flow, and runtime mechanics within PrintProof3D.

---

## 1. System Topology & Integration Boundaries

PrintProof3D exposes multiple programmatic and network boundaries to support client integrations (AI agents, static analyzers, and remote management tools):

```mermaid
graph TD
    %% Clients & Entry Points
    User[Developer / Slicer Client] -->|Native Imports| SDK[crates/sdk]
    CI[CI/CD Pipelines / CLI] -->|CLI Commands| CLI[crates/cli]
    Agent[AI Agent / Cursor] -->|MCP Line Protocol| MCP[MCP Server]
    RemoteClient[Remote Management App] -->|Axum HTTP REST| REST[crates/rest]

    %% Routing
    CLI -->|Imports| Core[crates/core]
    CLI -->|Imports| Printability[crates/printability]
    CLI -->|Imports| Plugins[crates/plugins]

    REST -->|Bearer Auth| Core
    REST -->|Routes Validation| Printability
    REST -->|Loads Hooks| Plugins

    MCP -->|Tools Engine| Core
    MCP -->|Tools Engine| Printability

    SDK -->|Conformance Suite| Adapters[crates/adapters]
    SDK -->|Imports| Core

    %% Infrastructure
    Printability -->|STL/G-code Geometry Checks| Core
    Adapters -->|Moonraker/OctoPrint/Serial| Core

    %% Sandboxed Plugins
    Plugins -->|Instantiates wasmi| Sandbox[Restricted Guest Sandbox]
    Sandbox -->|Guest Exec| Guest[example-plugin.wasm]
```

---

## 2. WebAssembly Memory Sandbox Flow

Custom validation rules are loaded as WASM byte slices and executed in a restricted memory sandbox utilizing the `wasmi` interpreter. Memory exchanges use raw pointer layouts (`alloc`, `dealloc`, and `validate` exports):

```mermaid
sequenceDiagram
    autonumber
    participant Host as PrintProof3D Host (PluginEngine)
    participant WASM as WebAssembly Instance (wasmi)
    participant Memory as Linear WASM Memory

    Host->>WASM: call alloc(input_len)
    WASM-->>Host: returns input_ptr (offset in linear memory)
    Host->>Memory: write input JSON string to input_ptr
    Host->>WASM: call validate(input_ptr, input_len)
    Note over WASM: Deserializes report,<br/>runs custom validation checks,<br/>serializes report to string,<br/>calls guest alloc() to store output
    WASM-->>Host: returns result_u64 (output_ptr << 32 | output_len)
    Host->>WASM: call dealloc(input_ptr, input_len)
    Host->>Memory: read output JSON string from output_ptr
    Host->>WASM: call dealloc(output_ptr, output_len)
    Host->>Host: Deserializes final report and merges
```

---

## 3. Conformance Test Suite Flowchart

The automated SDK conformance suite validates that third-party `PrinterAdapter` implementations preserve state invariants and respond correctly under normal and simulated failure paths:

```mermaid
flowchart TD
    Start([Start Suite]) --> Connect[Call connect]
    Connect -->|Err| FailConnect([Fail: Connection Error])
    Connect -->|Ok| Status[Call get_status]
    
    Status -->|Err| FailStatus([Fail: Status Error])
    Status -->|Ok: Telemetry| CheckState{Is telemetry.state valid?}
    CheckState -->|No| FailState([Fail: Empty State])
    CheckState -->|Yes| Control[Execute Control Loop]

    Control --> Pause[Call pause_job]
    Pause --> Resume[Call resume_job]
    Resume --> Cancel[Call cancel_job]
    
    Cancel -->|Err| FailControl([Fail: Control Command Failed])
    Cancel -->|Ok| Disconnect[Call disconnect]
    Disconnect -->|Err| FailDisconnect([Fail: Disconnect Failed])
    Disconnect -->|Ok| Pass([Pass: Conformance Confirmed])
```

---

## 4. Decoupled Data Flow Pipeline

Data validations flow through independent analysis and filtration layers:

```mermaid
graph LR
    Input[STL Mesh / G-code File] --> Parser[printproof3d-printability]
    Parser --> Profile[Compare to Printer/Material JSON Profiles]
    Profile --> RawReport[Generate baseline ValidationReport]
    RawReport --> Filter{Has Custom Rules WASM Plugin?}
    Filter -->|No| Output[Output final JSON report]
    Filter -->|Yes| Plugins[wasmi Sandboxed validation]
    Plugins --> Output
```
