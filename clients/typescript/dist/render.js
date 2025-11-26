import { formatFilePosition, formatFileLineRange, formatFileRangeStart, } from "./position-utils.js";
/**
 * Renders a list of files as a flat list.
 * @example
 * foo.ts
 * bar/foo.ts
 */
export function renderListFiles(result) {
    return result.join("\n");
}
/**
 * Renders find file results as a flat file list with matching scores.
 * @example
 * foo.ts (100%)
 * bar/foo.ts (83%)
 */
export function renderFindFile(result) {
    if (result.matches.length === 0) {
        return "(no files found)";
    }
    return result.matches
        .map((match) => {
        const percentage = Math.round(match.score * 100);
        return `${match.path} (${percentage}%)`;
    })
        .join("\n");
}
/**
 * Renders project layout as a directory tree using unicode characters.
 * @example
 * src (100%)
 * ├─ bar
 *    └─ foo
 */
export function renderProjectLayout(result) {
    if (!result.root) {
        return "(no project layout available)";
    }
    const lines = [];
    function renderNode(node, prefix, isLast) {
        const connector = isLast ? "└─ " : "├─ ";
        const countSuffix = node.file_count !== undefined ? ` (${node.file_count})` : "";
        lines.push(`${prefix}${connector}${node.name}${countSuffix}`);
        if (node.children && node.children.length > 0) {
            const childPrefix = prefix + (isLast ? "   " : "│  ");
            node.children.forEach((child, index) => {
                const childIsLast = index === node.children.length - 1;
                renderNode(child, childPrefix, childIsLast);
            });
        }
    }
    // Render root without prefix
    const countSuffix = result.root.file_count !== undefined ? ` (${result.root.file_count})` : "";
    lines.push(`${result.root.name}${countSuffix}`);
    if (result.root.children && result.root.children.length > 0) {
        result.root.children.forEach((child, index) => {
            const isLast = index === result.root.children.length - 1;
            renderNode(child, "", isLast);
        });
    }
    return lines.join("\n");
}
/**
 * Renders a source fragment with line numbers.
 * The padding is computed based on the maximum line number.
 *
 * @param sourceCode - The source code to render
 * @param startLine - The starting line number (0-indexed)
 * @returns Formatted source with line numbers
 */
function renderSourceFragment(sourceCode, startLine, leftPad = 0) {
    const lines = sourceCode.split("\n");
    if (lines.length > 0 && lines[lines.length - 1] === "") {
        lines.pop();
    }
    const maxLineNum = startLine + lines.length - 1;
    const maxLineNumWidth = maxLineNum.toString().length;
    return lines
        .map((line, index) => {
        const padding = " ".repeat(leftPad);
        const lineNum = (startLine + index + 1)
            .toString()
            .padStart(maxLineNumWidth);
        return `${padding}${lineNum} ${line}`;
    })
        .join("\n");
}
/**
 * Renders find references results: selected definition followed by list of references.
 * @example
 * selected: bar.ts:100:1: Bar
 * bar.ts:30:7
 */
export function renderFindReferences(result) {
    const lines = [];
    // Add selected identifier
    const sel = result.selected_identifier;
    const kind = sel.kind || "symbol";
    lines.push(`${formatFileRangeStart(sel.file_range)}: ${kind} ${sel.name}`);
    // Add references
    if (result.references.length === 0) {
        lines.push("(no references found)");
        return lines.join("\n");
    }
    for (const idx in result.references) {
        const ref = result.references[idx];
        lines.push(` <= ${formatFilePosition(ref)}`);
        const ctx = result.context && result.context[idx];
        if (ctx) {
            const rendered = renderSourceFragment(ctx.source_code, ctx.range.range.start.line, 4);
            lines.push(rendered);
        }
    }
    return lines.join("\n");
}
/**
 * Renders find referred symbols results as a list of symbols.
 * @example
 * bar.ts:30:7: class Bar
 */
export function renderFindReferencedSymbols(result) {
    const lines = [];
    // Render workspace symbols
    if (result.workspace_symbols.length > 0) {
        for (const sym of result.workspace_symbols) {
            const ref = sym.reference;
            const refKind = ref.kind ?? "symbol";
            lines.push(`${formatFileRangeStart(ref.file_range)}: ${refKind} ${ref.name}`);
            for (const def of sym.definitions) {
                const defKind = def.kind ?? "symbol";
                lines.push(` => ${formatFileLineRange(def.file_range)}: ${defKind} ${def.name}`);
            }
        }
    }
    // Render external symbols
    if (result.external_symbols.length > 0) {
        for (const ext of result.external_symbols) {
            const kind = ext.kind ?? "symbol";
            lines.push(`${formatFileRangeStart(ext.file_range)}: ${kind} ${ext.name} (external)`);
        }
    }
    // Render not found symbols
    if (result.not_found.length > 0) {
        for (const notFound of result.not_found) {
            const kind = notFound.kind ?? "symbol";
            lines.push(`${formatFileRangeStart(notFound.file_range)}: ${kind} ${notFound.name} (not found)`);
        }
    }
    if (lines.length === 0) {
        return "(no referenced symbols found)";
    }
    return lines.join("\n");
}
/**
 * Renders find identifier results as a list of identifiers.
 * @example
 * bar.ts:30:7: class Bar
 */
export function renderFindIdentifier(result) {
    if (result.identifiers.length === 0) {
        return "(no identifiers found)";
    }
    return result.identifiers
        .map((id) => {
        const kind = id.kind ?? "symbol";
        return `${formatFileRangeStart(id.file_range)}: ${kind} ${id.name}`;
    })
        .join("\n");
}
/**
 * Renders find definition results: selected identifier followed by definitions with source context.
 * @example
 * selected: bar.ts:100:1: Bar
 * bar.ts:32:7: class Bar
 *  99 | class Bar {
 * 100 | }
 */
export function renderFindDefinition(result) {
    const lines = [];
    // Add selected identifier
    const sel = result.selected_identifier;
    const kind = sel.kind || "symbol";
    lines.push(`${formatFileRangeStart(sel.file_range)}: ${kind} ${sel.name}`);
    // Add definitions
    if (result.definitions.length === 0) {
        lines.push("(no definitions found)");
        return lines.join("\n");
    }
    for (const idx in result.definitions) {
        const def = result.definitions[idx];
        lines.push(` => ${formatFilePosition(def)}`);
        const ctx = result.source_code_context && result.source_code_context[idx];
        if (ctx) {
            const rendered = renderSourceFragment(ctx.source_code, ctx.range.range.start.line, 4);
            lines.push(rendered);
        }
    }
    return lines.join("\n");
}
/**
 * Renders definitions in file as a list of definitions.
 * @example
 * bar.ts:12:9: function foo
 * bar.ts:100:7: class Bar
 */
export function renderDefinitionsInFile(result) {
    if (result.length === 0) {
        return "(no definitions found)";
    }
    return result
        .map((def) => {
        return `${formatFilePosition(def.identifier_position)}: ${def.kind} ${def.name}`;
    })
        .join("\n");
}
/**
 * Renders read source results as a source fragment with line numbers.
 * @example
 *  1 | class Bar {
 *  2 | }
 *
 * @param result - The read source result from the API
 * @param range - Optional 0-indexed LSP range that was used to read the source
 * @returns Formatted source with 1-indexed line numbers for display
 */
export function renderReadSource(result, range) {
    return renderSourceFragment(result.source_code, range?.start.line || 0);
}
/**
 * Renders a directory tree as plain text using unicode box-drawing characters.
 * @param node - The root directory node to render
 * @param showCounts - Whether to show file counts per directory
 * @param prefix - Current indentation prefix (used internally for recursion)
 * @param isLast - Whether this is the last child in its parent (used internally for recursion)
 * @returns Array of formatted lines representing the tree structure
 */
export function renderTreeAsPlainText(node, showCounts, prefix = "", isLast = true) {
    const lines = [];
    // Render current node
    const count = showCounts && node.file_count ? ` (${node.file_count} files)` : "";
    const connector = prefix ? (isLast ? "└── " : "├── ") : "";
    lines.push(`${prefix}${connector}${node.name}/${count}`);
    // Render children
    if (node.children && node.children.length > 0) {
        const childPrefix = prefix + (isLast ? "    " : "│   ");
        node.children.forEach((child, index) => {
            const childIsLast = index === node.children.length - 1;
            lines.push(...renderTreeAsPlainText(child, showCounts, childPrefix, childIsLast));
        });
    }
    return lines;
}
