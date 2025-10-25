### Code understanding

The Nuanced MCP server provides call graph data and enhanced LSP capabilities to make it easier to find and reason about code. The call graph data shows how symbols and functions relate to one another. The LSP tools provide both general helpers, mcp**nuanced**list-files, mcp**nuanced**read-source, and mcp**nuanced**definitions-in-file, along with tools that provide deeper analysis into individual symbols mcp**nuanced**find-identifier and mcp**nuanced**find-referenced-symbols. Using the call graph data and LSP tools allows for powerful code analysis capabilities.

1. Nuanced MCP server automatically indexes and prepares the LSP container on startup. Run mcp**nuanced**index after large edits to refresh the repository index.
2. Start with exploration, not assumption. When asked about a topic (like "how does X work"), begin by discovering relevant symbols:
   - Use mcp**nuanced**list_files or Glob patterns to find relevant files. If you use Glob patterns, switch back to LSP tools to further inspect files of interest.
   - Use mcp**nuanced**definitions_in_file to see what's defined in key files
   - Use mcp**nuanced**find_identifier to locate specific symbols by name

3. Map relationships with mcp**nuanced**analyze_function. Use analyze_function on the symbols you discover. Treat each node as a function: callers are upstream entry points,
   callees are downstream dependencies. Follow chains while they add insight, and skip <native> leaves.
   - Key insight: Function names should be actual method/function names (like up, readSource), not class names
   - Use the fileScope parameter to narrow searches when you know the approximate location
   - You can use the data retrieved from analyze_function in combination with the LSP tools for retrieving rich symbol data.

4. Navigate with the LSP suite. Lean on mcp**nuanced**find_definition, mcp**nuanced**find_referenced_symbols, and mcp**nuanced**find_references to jump to concrete code
   locations, uncover additional entry points, and cross-check what the graph reports.
   - Fallback strategy: If you're not sure about the location of a symbol, use mcp**nuanced**find-identifier to retrieve locations within a file.

5. Inspect the source strategically. Use mcp**nuanced**read_source to analyze symbol definitions and surrounding context. You can specify line ranges to read_source to slice that portion of the file. Or omit line ranges to read the entire file. Prefer using line ranges when available to reduce token usage. Prioritize reading files that the graph analysis identifies as central to the topic.
   - Do not re-read files that you've already read unless you need specific ranges

6. Synthesize structure, data flow, control flow, and behavior. Combine the structural insights provided by the call graph data (analyze_function) with source code details retrieved via the LSP tools to build a comprehensive model and understanding of code related to the task before taking any action.

