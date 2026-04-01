# API Tester - Postman Alternative

A comprehensive API testing tool built as part of the EasySSH project. Features HTTP client, WebSocket support, collections management, environments, history tracking, and Postman import/export.

## Features

### 1. HTTP Client
- Full HTTP method support: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- Request headers, query parameters, body editing
- Multiple body types: JSON, XML, Form, Text, Multipart, Binary
- Authentication: Basic, Bearer Token, API Key, OAuth2, Digest

### 2. Response Viewing
- Syntax highlighting for JSON, XML, HTML
- Pretty print and raw view modes
- Response headers and cookies inspection
- Download and copy response body

### 3. Collections Management
- Organize API requests into collections
- Nested folders support
- Drag and drop organization
- Search across collections

### 4. Environments & Variables
- Multiple environment support (Development, Staging, Production)
- Variable substitution with `{{variable}}` syntax
- Global and collection-level variables
- Active environment switching

### 5. History
- Automatic request history tracking
- Replay requests from history
- Search and filter history
- Convert history to saved requests

### 6. Automated Testing
- Postman-style test scripts
- Assertions: status codes, response body, headers, JSON paths
- Test results visualization

### 7. WebSocket Client
- Connect to WebSocket servers
- Send and receive messages
- Message history with timestamps
- Text and JSON message support

### 8. gRPC Support (Planned)
- gRPC method discovery via reflection
- Protocol Buffer message editing
- Unary and streaming calls

### 9. Import/Export
- Import Postman collections (v2.1)
- Import Postman environments
- Import cURL commands
- Export to Postman format
- Export to cURL

## Architecture

```
api-tester/
├── api-core/          # Rust core library
│   ├── src/
│   │   ├── types.rs       # Core data types
│   │   ├── client.rs      # HTTP client
│   │   ├── websocket.rs   # WebSocket client
│   │   ├── grpc.rs        # gRPC client (planned)
│   │   ├── database.rs    # SQLite storage
│   │   ├── test_runner.rs # Test script runner
│   │   ├── import_export.rs # Import/Export formats
│   │   ├── collection.rs  # Collection management
│   │   ├── environment.rs # Environment variables
│   │   └── history.rs     # History management
│   └── Cargo.toml
├── api-tauri/         # Tauri integration
│   ├── src/
│   │   ├── commands.rs    # Tauri commands
│   │   └── lib.rs
│   └── Cargo.toml
└── api-ui/            # React frontend
    ├── src/
    │   ├── components/
    │   │   ├── Sidebar.tsx
    │   │   ├── RequestBuilder.tsx
    │   │   ├── ResponseViewer.tsx
    │   │   ├── WebSocketClient.tsx
    │   │   └── ApiTester.tsx
    │   ├── stores/
    │   │   └── apiTesterStore.ts
    │   └── utils/
    │       └── tauriCommands.ts
    └── package.json
```

## Usage

### Building

```bash
# Build the Rust core
cd api-tester/api-core
cargo build

# Build the Tauri integration
cd api-tester/api-tauri
cargo build

# Build the UI
cd api-tester/api-ui
npm install
npm run build
```

### Integration

Add to your Tauri app:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(api_tester_tauri::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Use in React:

```tsx
import { ApiTester } from 'api-tester-ui';

function App() {
  return <ApiTester />;
}
```

## Test Script Syntax

The API Tester supports Postman-style test scripts:

```javascript
// Status code assertion
pm.test("Status code is 200", function () {
  pm.response.to.have.status(200);
});

// Response time assertion
pm.test("Response time is less than 500ms", function () {
  pm.expect(pm.response.responseTime).to.be.below(500);
});

// JSON body assertion
pm.test("Response has correct structure", function () {
  var jsonData = pm.response.json();
  pm.expect(jsonData).to.have.property("id");
  pm.expect(jsonData.id).to.equal(123);
});

// Header assertion
pm.test("Content-Type is JSON", function () {
  pm.response.to.have.header("Content-Type");
  pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});
```

## License

MIT
