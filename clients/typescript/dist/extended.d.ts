import type { FindFileResult, MatchType, ProjectLayoutResult } from "./types.js";
/**
 * Build a project layout tree from a list of files.
 * This function analyzes file paths and creates a hierarchical directory structure
 * with optional file counts per directory.
 */
export declare function projectLayout(files: string[], opts?: {
    directory?: string;
    maxDepth?: number;
    showCounts?: boolean;
    formatAsPlainText?: boolean;
}): ProjectLayoutResult;
/**
 * Find files matching a pattern using both glob and fuzzy matching strategies.
 * This combines glob/micromatch for wildcard patterns with Levenshtein distance
 * for fuzzy matching to handle typos and partial matches.
 */
export declare function findFile(files: string[], pattern: string, opts?: {
    directory?: string;
} & MatchOptions): Promise<FindFileResult>;
export interface Match<T> {
    data: T;
    score: number;
    match_type: MatchType;
}
export interface MatchOptions {
    maxDistance?: number;
    maxResults?: number;
}
/**
 * Generic pattern matching function that combines glob and fuzzy matching strategies.
 * This function can match any type of data by extracting a string key for comparison.
 */
export declare function match<T>(values: T[], key: (value: T) => string, pattern: string, opts?: MatchOptions): Promise<Match<T>[]>;
//# sourceMappingURL=extended.d.ts.map