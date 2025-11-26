import type { FilePosition, FileRange, LspPosition, LspRange } from "./types.js";
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
export declare function parseFilePosition(input: string, indexBase?: number): {
    path: string;
    position?: LspPosition;
};
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
export declare function parseFileRange(input: string, indexBase?: number): {
    path: string;
    range?: LspRange;
};
export declare function formatPosition(pos: LspPosition): string;
export declare function formatLineRange(range: LspRange): string;
export declare function formatFilePosition(file_pos: FilePosition): string;
export declare function formatFileLineRange(file_range: FileRange): string;
export declare function formatFileRangeStart(file_range: FileRange): string;
//# sourceMappingURL=position-utils.d.ts.map