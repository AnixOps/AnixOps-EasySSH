# EasySSH API Design Consistency Report

**Report Date:** 2026-04-02
**Scope:** Core Library, Tauri Commands, FFI Bindings, WebSocket Events
**Versions Analyzed:** 0.3.0

---

## Executive Summary

This report analyzes the EasySSH API design consistency across the following areas:
1. Tauri command naming conventions
2. Error code design and handling
3. Parameter serialization/deserialization
4. API version control strategy
5. Async API return types
6. Event naming conventions
7. Documentation vs implementation alignment

**Overall Consistency Grade:** C+

The API exhibits mixed consistency - the core data models are well-structured, but there are notable inconsistencies in naming conventions, error handling patterns, and event naming across different modules.

---

## 1. Tauri Command Naming Convention Analysis

### Current Patterns Found

| Module | Naming Pattern | Example |
|--------|----------------|---------|
| recording_commands | `recording_<action>` | `recording_start`, `recording_stop` |
| kubernetes_tauri | `k8s_<action>_<resource>` | `k8s_get_pods`, `k8s_scale_deployment` |
| api-tauri | `<action>_<resource>` | `execute_request`, `save_collection` |
| git_manager | `<action>_<resource>` | `clone_repo`, `stage` |

### Inconsistencies Detected

**ISSUE-001: Prefix Inconsistency**
- **Problem:** Some modules use domain prefixes (`recording_`, `k8s_`) while others use flat naming (`save_collection`, `execute_request`)
- **Impact:** Frontend code cannot predict command names without checking documentation
- **Example:**
  ```rust
  // Has prefix
  recording_start(...)
  k8s_get_pods(...)

  // No prefix
  save_collection(...)
  execute_request(...)
  ```

**ISSUE-002: Verb Choice Inconsistency**
- **Problem:** Mix of CRUD-style and action-style verbs
- **Examples:**
  - `get_` vs `list_` (both return collections)
  - `delete_` vs `remove_` vs `uninstall_`
  - `create_` vs `add_` vs `save_`

**ISSUE-003: Resource Name Pluralization**
- Inconsistent pluralization:
  - `k8s_get_clusters` (plural)
  - `get_collection` (singular)
  - `list_collections` (plural)

### Recommended Naming Convention

```
<domain>_<action>_<resource>

Actions:
- get_    : Retrieve single item
- list_   : Retrieve multiple items (with pagination)
- create_ : Create new item
- update_ : Modify existing item
- delete_ : Remove item
- execute_: Run operation without side effects on data
- start_  : Begin process/operation
- stop_   : End process/operation
```

---

## 2. Error Code Design Consistency

### Current State: LiteError Enum

```rust
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LiteError {
    #[error("error-database")]
    Database(String),

    #[error("error-crypto")]
    Crypto(String),

    #[error("ssh-connection-failed")]
    SshConnectionFailed { host: String, port: u16, message: String },

    // ... 30+ variants
}
```

### Inconsistencies Detected

**ISSUE-004: Error Key Format Inconsistency**
- **Kebab-case:** `error-database`, `error-crypto`
- **Snake-case:** `ssh_connection_failed` (in struct name, not error key)
- **Mixed in same enum:** Some use `error-` prefix, others don't

**ISSUE-005: Error Response Format Inconsistency**

Tauri commands use different error conversion approaches:

```rust
// Approach 1: String conversion (loss of error context)
#[tauri::command]
pub async fn k8s_get_clusters(...) -> Result<Vec<K8sCluster>, String> {
    // ... map_err(|e| e.to_string())
}

// Approach 2: LiteError serialization (translation keys)
// Core functions return LiteError which serializes to translation keys
```

**ISSUE-006: Error Code vs Display Message Confusion**

```rust
#[error("error-database")]  // This is the error key/translation key
Database(String),

// The to_string() returns "error-database", not the actual message
// This is intentional for translation, but confusing for API consumers
```

### Error Code Mapping Analysis

| Error Variant | Translation Key | HTTP-Style Code | Consistent? |
|---------------|-------------------|-----------------|-------------|
| Database | error-database | DB_ERROR | No prefix |
| SshConnectionFailed | error-connection-failed | CONN_FAILED | Different pattern |
| SshAuthFailed | connection-auth-failed | AUTH_FAILED | Missing 'error-' prefix |
| ServerNotFound | error-not-found | NOT_FOUND | Generic key |
| GroupNotFound | group-not-found | NOT_FOUND | No 'error-' prefix |
| FeatureNotAvailable | error-feature-not-available | FEATURE_NA | OK |

### Recommendations

1. **Standardize all error keys** to format: `error-<category>-<specific>`
2. **Align translation keys** with error display messages
3. **Provide structured error responses** for Tauri:
   ```rust
   #[derive(Serialize)]
   struct ApiError {
       code: String,        // Machine-readable: "error-ssh-connection-failed"
       message: String,     // Human-readable: "Failed to connect to {host}:{port}"
       details: Option<Value>, // Structured context
       request_id: String,
   }
   ```

---

## 3. Parameter Serialization/Deserialization

### Current Patterns

**Pattern 1: Direct Struct Deserialization (Good)**
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct StartRecordingRequest {
    pub width: u32,
    pub height: u32,
    pub title: Option<String>,
    // ...
}

#[tauri::command]
pub async fn recording_start(
    state: State<'_, RecordingStateWrapper>,
    request: StartRecordingRequest,  // Clean deserialization
) -> Result<String, String>
```

**Pattern 2: Individual Parameters (Verbose)**
```rust
#[tauri::command]
pub async fn k8s_get_pod_logs(
    cluster_id: String,
    namespace: String,
    pod_name: String,
    follow: bool,
    tail_lines: Option<i64>,
    since_seconds: Option<i64>,
    timestamps: bool,
    previous: bool,
    container: Option<String>,  // 9 parameters!
) -> Result<String, String>
```

**Pattern 3: Raw JSON Value (Flexible but Untyped)**
```rust
// Some FFI functions use JSON strings for complex data
pub unsafe extern "C" fn easyssh_add_server(
    handle: *mut EasySSHAppState,
    json_config: *const c_char,  // Raw JSON
) -> c_int
```

### Inconsistencies Detected

**ISSUE-007: Parameter Style Inconsistency**
- Some commands use request structs (clean, extensible)
- Some use individual parameters (verbose, brittle)
- FFI layer uses raw JSON strings (no compile-time checking)

**ISSUE-008: Optional Parameter Handling**
```rust
// Kubernetes: Uses Option<T> for optional params
tail_lines: Option<i64>

// Recording: Uses Option<T> in request struct
pub title: Option<String>

// API Tester: Some params have defaults implied but not explicit
```

### Recommendations

1. **Standardize on Request/Response structs** for commands with >3 parameters
2. **Use builder pattern** for complex operations
3. **Maintain backward compatibility** when adding fields (all new fields should be `Option<T>` with defaults)

---

## 4. API Version Control Strategy

### Documentation vs Reality

**API Design Document (docs/architecture/api-design.md):**
```yaml
# Specifies clear versioning strategy
POST /api/v1/auth/login
GET /api/v1/servers
```

**Actual Implementation:**
- Tauri commands are **not versioned**
- FFI functions use `easyssh_` prefix (flat namespace)
- No runtime API version negotiation

### Inconsistencies Detected

**ISSUE-009: Version Strategy Gap**

| Component | Versioned | Strategy |
|-----------|-----------|----------|
| REST API (documented) | Yes | URL path /api/v1/ |
| Tauri Commands | No | Flat namespace |
| FFI Interface | No | flat namespace |
| WebSocket Events | No | flat namespace |
| Data Models | Partial | serde attributes |

**ISSUE-010: Breaking Change Risk**
- Tauri commands bind directly to internal data structures
- No deprecation mechanism
- No backward compatibility layer

### Recommendations

1. **Add version attribute** to Tauri commands:
   ```rust
   #[tauri::command(version = "1.0")]
   pub async fn get_servers(...) { }
   ```

2. **Implement feature flags** for API evolution:
   ```rust
   #[cfg(feature = "api-v2")]
   #[tauri::command]
   pub async fn get_servers_v2(...) { }
   ```

3. **Version the FFI layer**:
   ```rust
   easyssh_v1_init()
   easyssh_v2_init() // With extended capabilities
   ```

---

## 5. Async API Return Types

### Current Patterns

| Pattern | Usage | Example |
|---------|-------|---------|
| `Result<T, String>` | Most Tauri commands | `Result<Vec<Server>, String>` |
| `Result<T, LiteError>` | Core library | `Result<SessionMetadata, LiteError>` |
| Raw pointers | FFI | `*mut c_char` |
| Integer codes | FFI | `c_int` (0 = success, -1 = error) |

### Inconsistencies Detected

**ISSUE-011: Error Type Inconsistency**

```rust
// Core library: Rich error type
pub async fn ssh_connect(...) -> Result<SessionMetadata, LiteError>

// Tauri layer: String errors only
#[tauri::command]
pub async fn recording_start(...) -> Result<String, String>

// FFI layer: Integer codes
#[no_mangle]
pub extern "C" fn easyssh_add_server(...) -> c_int
```

**ISSUE-012: Void Return Inconsistency**
```rust
// Some commands return () for success
pub async fn recording_pause(...) -> Result<(), String>

// Others return empty string or specific value
pub async fn recording_stop(...) -> Result<RecordingMetadata, String>
```

**ISSUE-013: Option vs Result for "Not Found"**
```rust
// Core: Uses Result with specific error
pub fn get_server(...) -> Result<ServerRecord, LiteError>  // Returns ServerNotFound error

// Some Tauri: Uses Option
pub async fn recording_get_state(...) -> Result<Option<RecordingState>, String>
```

### Recommendations

1. **Standardize on structured errors** for all public APIs:
   ```rust
   #[derive(Serialize)]
   struct ApiResponse<T> {
       success: bool,
       data: Option<T>,
       error: Option<ApiError>,
   }
   ```

2. **Use Result consistently** - avoid mixing `Option` and `Result` for similar operations

3. **Create FFI error code enum**:
   ```rust
   pub const EASYSSH_OK: c_int = 0;
   pub const EASYSSH_ERROR_INVALID_PARAM: c_int = -1;
   pub const EASYSSH_ERROR_DATABASE: c_int = -2;
   // ... etc
   ```

---

## 6. Event Naming Convention

### Current Event Names Found

| Source | Event Name | Context |
|--------|------------|---------|
| auto_update | `update-event` | General update events |
| kubernetes | `k8s-log` | Pod log streaming |
| kubernetes | `k8s-event` | Kubernetes events |

### Inconsistencies Detected

**ISSUE-014: Naming Pattern Inconsistency**

Current patterns observed:
- `update-event` (kebab-case, generic)
- `k8s-log` (kebab-case, domain-prefixed)
- `k8s-event` (kebab-case, domain-prefixed)

**ISSUE-015: Event Name Collision Risk**
- Generic names like `update-event` could conflict
- No namespacing convention documented

**ISSUE-016: Event Payload Inconsistency**
```rust
// k8s-log: Manual JSON construction
window.emit("k8s-log", serde_json::json!({
    "podName": pod_name,
    "container": container,
    "log": log,
}));

// k8s-event: Direct struct emission
window.emit("k8s-event", event);  // K8sEvent struct

// update-event: Enum serialization
app_handle.emit_all("update-event", event);  // UpdateUiEvent enum
```

### Recommended Event Naming Convention

```
<domain>:<resource>:<action>

Examples:
- k8s:pod:log
- k8s:cluster:event
- update:download:progress
- update:install:completed
- ssh:session:connected
- ssh:session:disconnected
```

Benefits:
1. **Hierarchical namespacing** prevents collisions
2. **Predictable pattern** for frontend subscriptions
3. **Wildcards supported** (e.g., `k8s:pod:*` for all pod events)

---

## 7. API Documentation vs Implementation Consistency

### Alignment Analysis

| Document Section | Implementation Status | Alignment |
|------------------|----------------------|-----------|
| Error Codes (docs/architecture/api-design.md) | Partial | 60% |
| REST API Endpoints | Not implemented (Pro only) | N/A |
| WebSocket Protocol | Partial | 40% |
| Data Models | Mostly aligned | 85% |
| Authentication | Partial | 50% |

### Specific Mismatches

**ISSUE-017: Error Code Mismatch**

| Document Specification | Implementation | Match? |
|------------------------|----------------|--------|
| `validation_error` | `error-database` | No |
| `token_expired` | Not implemented | N/A |
| `permission_denied` | Not implemented | N/A |
| `sync_conflict` | Not implemented | N/A |

**ISSUE-018: Data Model Mismatch**

Document shows:
```rust
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub auth_data_encrypted: Vec<u8>,
    // ...
}
```

Actual implementation:
```rust
// db.rs
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,  // Note: i64, not u16
    pub username: String,
    pub auth_type: String,
    pub auth_data: Option<String>,  // Not Vec<u8>, not encrypted at rest
    // ...
}
```

**ISSUE-019: Missing API Endpoints**

Documented but not implemented:
- `/api/v1/auth/login` - No REST API server in current codebase
- `/api/v1/sync/*` - Sync FFI exists but no REST endpoints
- `/api/v1/audit/*` - Audit module exists but no REST endpoints

---

## Summary of Issues

| ID | Category | Severity | Description |
|----|----------|----------|-------------|
| ISSUE-001 | Naming | Medium | Command prefix inconsistency |
| ISSUE-002 | Naming | Low | Verb choice inconsistency |
| ISSUE-003 | Naming | Low | Pluralization inconsistency |
| ISSUE-004 | Error | High | Error key format inconsistency |
| ISSUE-005 | Error | High | Error response format varies |
| ISSUE-006 | Error | Medium | Error code vs display confusion |
| ISSUE-007 | Parameters | Medium | Parameter style inconsistency |
| ISSUE-008 | Parameters | Low | Optional param handling |
| ISSUE-009 | Versioning | High | No API versioning in Tauri |
| ISSUE-010 | Versioning | High | Breaking change risk |
| ISSUE-011 | Return Types | High | Error type inconsistency |
| ISSUE-012 | Return Types | Low | Void return patterns |
| ISSUE-013 | Return Types | Medium | Option vs Result |
| ISSUE-014 | Events | Medium | Event naming inconsistency |
| ISSUE-015 | Events | Medium | Collision risk |
| ISSUE-016 | Events | Low | Payload inconsistency |
| ISSUE-017 | Docs | High | Error code mismatch |
| ISSUE-018 | Docs | High | Data model mismatch |
| ISSUE-019 | Docs | High | Endpoints not implemented |

---

## Recommendations Summary

### High Priority (Immediate Action)

1. **Standardize Error Handling**
   - Create `ApiError` struct for all Tauri commands
   - Align LiteError translation keys with documented error codes
   - Implement structured error responses

2. **Fix API Documentation**
   - Update docs to match actual data models
   - Remove or mark as "planned" unimplemented endpoints
   - Document actual error codes returned

3. **Implement API Versioning**
   - Add versioning to Tauri commands
   - Create compatibility layer for breaking changes

### Medium Priority (Next Sprint)

4. **Standardize Command Naming**
   - Adopt `<domain>_<action>_<resource>` pattern
   - Create naming convention documentation
   - Refactor existing commands (with deprecation)

5. **Standardize Event Naming**
   - Adopt `<domain>:<resource>:<action>` pattern
   - Document all emitted events
   - Add event payload schemas

### Low Priority (Ongoing)

6. **Parameter Consistency**
   - Use request structs for >3 parameters
   - Document default values

7. **Return Type Consistency**
   - Use `Result<T, ApiError>` pattern everywhere
   - Remove `Option` returns for "not found" scenarios

---

## Appendix: Current API Surface

### Tauri Commands by Module

| Module | Commands Count | Naming Pattern |
|--------|----------------|----------------|
| recording_commands | 15 | `recording_<action>` |
| kubernetes_tauri | 32 | `k8s_<action>_<resource>` |
| api-tauri | 25 | `<action>_<resource>` |

### FFI Functions by Module

| Module | Functions | Pattern |
|--------|-----------|---------|
| ffi.rs | 8 | `easyssh_<action>` |
| sync_ffi.rs | 12 | `sync_manager_<action>` |
| log_monitor_ffi.rs | 6 | `log_monitor_<action>` |
| i18n_ffi.rs | 14 | `easyssh_i18n_<action>` |

### Events Emitted

| Source | Event | Payload Type |
|--------|-------|--------------|
| kubernetes_tauri | `k8s-log` | JSON object |
| kubernetes_tauri | `k8s-event` | K8sEvent struct |
| auto_update | `update-event` | UpdateUiEvent enum |

---

*Report generated by Claude Code API Consistency Analyzer*
