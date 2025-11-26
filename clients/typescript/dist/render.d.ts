import type { DefinitionsInFileResult, FindDefinitionResult, FindFileResult, FindIdentifierResult, FindReferencedSymbolsResult, FindReferencesResult, ListFilesResult, ProjectLayoutResult, ReadSourceResult, DirectoryNode, LspRange } from "./types.js";
/**
 * Renders a list of files as a flat list.
 * @example
 * foo.ts
 * bar/foo.ts
 */
export declare function renderListFiles(result: ListFilesResult): string;
/**
 * Renders find file results as a flat file list with matching scores.
 * @example
 * foo.ts (100%)
 * bar/foo.ts (83%)
 */
export declare function renderFindFile(result: FindFileResult): string;
/**
 * Renders project layout as a directory tree using unicode characters.
 * @example
 * src (100%)
 * ├─ bar
 *    └─ foo
 */
export declare function renderProjectLayout(result: ProjectLayoutResult): string;
/**
 * Renders find references results: selected definition followed by list of references.
 * @example
 * selected: bar.ts:100:1: Bar
 * bar.ts:30:7
 */
export declare function renderFindReferences(result: FindReferencesResult): string;
/**
 * Renders find referred symbols results as a list of symbols.
 * @example
 * bar.ts:30:7: class Bar
 */
export declare function renderFindReferencedSymbols(result: FindReferencedSymbolsResult): string;
/**
 * Renders find identifier results as a list of identifiers.
 * @example
 * bar.ts:30:7: class Bar
 */
export declare function renderFindIdentifier(result: FindIdentifierResult): string;
/**
 * Renders find definition results: selected identifier followed by definitions with source context.
 * @example
 * selected: bar.ts:100:1: Bar
 * bar.ts:32:7: class Bar
 *  99 | class Bar {
 * 100 | }
 */
export declare function renderFindDefinition(result: FindDefinitionResult): string;
/**
 * Renders definitions in file as a list of definitions.
 * @example
 * bar.ts:12:9: function foo
 * bar.ts:100:7: class Bar
 */
export declare function renderDefinitionsInFile(result: DefinitionsInFileResult): string;
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
export declare function renderReadSource(result: ReadSourceResult, range?: LspRange | null): string;
/**
 * Renders a directory tree as plain text using unicode box-drawing characters.
 * @param node - The root directory node to render
 * @param showCounts - Whether to show file counts per directory
 * @param prefix - Current indentation prefix (used internally for recursion)
 * @param isLast - Whether this is the last child in its parent (used internally for recursion)
 * @returns Array of formatted lines representing the tree structure
 */
export declare function renderTreeAsPlainText(node: DirectoryNode, showCounts: boolean, prefix?: string, isLast?: boolean): string[];
//# sourceMappingURL=render.d.ts.map