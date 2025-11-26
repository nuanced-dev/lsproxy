/**
 * Parse a file argument with optional position in format "filename" or "filename:line:column".
 * Returns a 0-indexed LSP position when position is provided.
 *
 * @example
 * parseFilePosition("foo/bar.ts", 1)
 * // Returns: { path: "foo/bar.ts" }
 *
 * parseFilePosition("foo/bar.ts:32:7", 1) // 1-indexed input
 * // Returns: { path: "foo/bar.ts", position: { line: 31, character: 6 } } (0-indexed)
 *
 * parseFilePosition("foo/bar.ts:31:6", 0) // 0-indexed input
 * // Returns: { path: "foo/bar.ts", position: { line: 31, character: 6 } } (0-indexed)
 *
 * @param input - File path optionally followed by :line:column
 * @param indexBase - The indexing base (0 for 0-indexed, 1 for 1-indexed)
 * @returns Object with path and optional 0-indexed LSP position ready for API
 * @throws Error if the position format is invalid
 */
export function parseFilePosition(input, indexBase = 1) {
    // Try to match the pattern filename:line:column
    // Use a regex that captures everything up to the last two colon-separated numbers
    const match = /^(.+):(\d+):(\d+)$/.exec(input.trim());
    if (!match) {
        // No position specified, just a file path
        return { path: input.trim() };
    }
    const path = match[1];
    const line = parseInt(match[2], 10);
    const column = parseInt(match[3], 10);
    if (Number.isNaN(line) || Number.isNaN(column)) {
        const indexDesc = indexBase === 0 ? "0-indexed" : "1-indexed";
        throw new Error(`Invalid position numbers. Both line and column must be integers (${indexDesc}).`);
    }
    if (line < indexBase || column < indexBase) {
        const indexDesc = indexBase === 0 ? "0-indexed" : "1-indexed";
        throw new Error(`Invalid position numbers. Line and column must be >= ${indexBase} (${indexDesc}).`);
    }
    // Convert to 0-indexed if input was 1-indexed
    const position = {
        line: indexBase === 0 ? line : Math.max(0, line - 1),
        character: indexBase === 0 ? column : Math.max(0, column - 1),
    };
    return { path, position };
}
/**
 * Parse a file argument with optional line range in format "filename" or "filename:startLine-endLine".
 * Returns a 0-indexed LSP range when range is provided.
 *
 * @example
 * parseFileRange("foo/bar.ts", 1)
 * // Returns: { path: "foo/bar.ts" }
 *
 * parseFileRange("foo/bar.ts:32-35", 1) // 1-indexed input
 * // Returns: { path: "foo/bar.ts", range: { start: { line: 31, character: 0 }, end: { line: 34, character: 0 } } }
 *
 * parseFileRange("foo/bar.ts:31-34", 0) // 0-indexed input
 * // Returns: { path: "foo/bar.ts", range: { start: { line: 31, character: 0 }, end: { line: 34, character: 0 } } }
 *
 * @param input - File path optionally followed by :startLine-endLine
 * @param indexBase - The indexing base (0 for 0-indexed, 1 for 1-indexed)
 * @returns Object with path and optional 0-indexed LSP range ready for API
 * @throws Error if the range format is invalid
 */
export function parseFileRange(input, indexBase = 1) {
    // Try to match the pattern filename:number-number
    const match = /^(.+):(\d+)-(\d+)$/.exec(input.trim());
    if (!match) {
        // No range specified, just a file path
        return { path: input.trim() };
    }
    const path = match[1];
    const startLine = parseInt(match[2], 10);
    const endLine = parseInt(match[3], 10);
    if (Number.isNaN(startLine) || Number.isNaN(endLine)) {
        const indexDesc = indexBase === 0 ? "0-indexed" : "1-indexed";
        throw new Error(`Invalid line range format. Line numbers must be integers (${indexDesc}).`);
    }
    if (startLine < indexBase || endLine < indexBase) {
        const indexDesc = indexBase === 0 ? "0-indexed" : "1-indexed";
        throw new Error(`Invalid line range. Lines must be >= ${indexBase} (${indexDesc}).`);
    }
    if (endLine < startLine) {
        throw new Error(`Invalid line range: endLine (${endLine}) must be >= startLine (${startLine}).`);
    }
    // Convert to 0-indexed LSP range with character: 0
    const range = indexBase === 0
        ? {
            start: { line: startLine, character: 0 },
            end: { line: endLine, character: 0 },
        }
        : {
            start: { line: Math.max(0, startLine - 1), character: 0 },
            end: { line: Math.max(0, endLine - 1), character: 0 },
        };
    return { path, range };
}
export function formatPosition(pos) {
    return `${pos.line + 1}:${pos.character + 1}`;
}
export function formatLineRange(range) {
    return `${range.start.line + 1}-${range.end.line + 1}`;
}
export function formatFilePosition(file_pos) {
    return `${file_pos.path}:${formatPosition(file_pos.position)}`;
}
export function formatFileLineRange(file_range) {
    return `${file_range.path}:${formatLineRange(file_range.range)}`;
}
export function formatFileRangeStart(file_range) {
    return `${file_range.path}:${formatPosition(file_range.range.start)}`;
}
