# lsp-wrapper Full Implementation Plan

## Completed ✅
1. Concurrent request handling (`Arc<LspProcess>` without outer Mutex)
2. Basic LSP endpoints: `/definition`, `/references`
3. Health check endpoint

## Remaining Work

### 1. Complete read-source endpoint with range support
**Current**: Only reads full file
**Needed**: Support optional line range parameter

```rust
#[derive(Deserialize)]
struct ReadSourceRequest {
    file_path: String,
    range: Option<Range>,  // Add this
}

struct Range {
    start: Position,
    end: Position,
}
```

Logic: Read file, if range provided, extract only those lines.

### 2. Complete find-identifier endpoint with position matching
**Current**: Returns raw ast-grep results
**Needed**:
- Filter identifiers by name
- If position provided: find exact match OR 3 closest matches
- Distance calculation: `(line_diff * 100 + char_diff)` prioritizing lines

See: `/Users/rewinfrey/code/nuanced/lsproxy/lsproxy/src/handlers/find_identifier.rs`
See: `/Users/rewinfrey/code/nuanced/lsproxy/lsproxy/src/handlers/utils.rs:24-66`

### 3. Complete find-referenced-symbols endpoint
**Current**: Gets symbols and references separately
**Needed**:
1. Get symbol at position using ast-grep (symbol config)
2. Get references within that symbol's range (reference config)
3. **For each reference**: Call `/definition` endpoint to get LSP definition
4. Categorize results:
   - Workspace symbols (with definitions)
   - External symbols (no definitions found)
   - Not found

See: `/Users/rewinfrey/code/nuanced/lsproxy/lsproxy/src/handlers/find_referenced_symbols.rs`
See: `/Users/rewinfrey/code/nuanced/lsproxy/lsproxy/src/lsp/manager/manager.rs:360-432`

Key logic:
```rust
// 1. Get symbol at position
let symbol = ast_grep_symbol_at_position(file, position)?;

// 2. Get references within symbol range
let references = ast_grep_references(file)
    .filter(|r| symbol.contains(r));

// 3. For each reference, call LSP definition
let mut results = vec![];
for reference in references {
    let definition = lsp.send_request("textDocument/definition", ...);
    results.push((reference, definition));
}

// 4. Categorize
categorize_by_workspace(results)
```

### 4. Add documentSymbol endpoint
Simple LSP forward - already started in code.

### 5. Wire up all routes in HttpServer
```rust
HttpServer::new(move || {
    App::new()
        .app_data(app_state.clone())
        .route("/health", web::get().to(health))
        .route("/lsp", web::post().to(lsp_handler))
        .route("/definition", web::post().to(definition_handler))
        .route("/references", web::post().to(references_handler))
        .route("/symbols", web::post().to(symbols_handler))
        .route("/list-files", web::get().to(list_files_handler))
        .route("/read-source", web::post().to(read_source_handler))
        .route("/find-identifier", web::post().to(find_identifier_handler))
        .route("/find-referenced-symbols", web::post().to(find_referenced_symbols_handler))
})
```

### 6. Add ast-grep config files to Docker images
Need to copy ast-grep configs to `/usr/src/ast_grep/` in all language images:
- `/usr/src/ast_grep/symbol/config.yml`
- `/usr/src/ast_grep/identifier/config.yml`
- `/usr/src/ast_grep/reference/config.yml`

### 7. Install ast-grep CLI in all language images
Add to base Dockerfile or each language Dockerfile:
```dockerfile
RUN curl -L https://github.com/ast-grep/ast-grep/releases/download/0.31.5/ast-grep-x86_64-unknown-linux-gnu.zip -o /tmp/ast-grep.zip && \
    unzip /tmp/ast-grep.zip -d /usr/local/bin && \
    chmod +x /usr/local/bin/ast-grep && \
    rm /tmp/ast-grep.zip
```

## Data Structures Needed

Create these in lsp-wrapper (port from main crate):

```rust
// From api_types.rs
struct Identifier {
    name: String,
    file_range: FileRange,
}

struct FileRange {
    path: String,
    range: Range,
}

struct Range {
    start: Position,
    end: Position,
}

struct Position {
    line: u32,
    character: u32,
}

// Conversion from ast-grep results to these types
```

## Testing Strategy

1. Build lsp-wrapper binary
2. Test locally with sample workspaces
3. Build Docker images with lsp-wrapper
4. Test full container flow
5. Integration tests with base Rust service

## Estimated Effort

- Read-source with range: 30 min
- Find-identifier with matching: 1-2 hours
- Find-referenced-symbols: 2-3 hours
- Wire up routes: 15 min
- Add ast-grep to Dockerfiles: 1 hour
- Testing: 2-3 hours

**Total**: ~8-10 hours of focused work
