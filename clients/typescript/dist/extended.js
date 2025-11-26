import { renderTreeAsPlainText } from "./render.js";
/**
 * Build a project layout tree from a list of files.
 * This function analyzes file paths and creates a hierarchical directory structure
 * with optional file counts per directory.
 */
export function projectLayout(files, opts) {
    const { directory, maxDepth = 3, showCounts = true, formatAsPlainText = false, } = opts ?? {};
    // Filter by directory if specified
    if (directory) {
        const normalizedDir = directory.endsWith("/") ? directory : directory + "/";
        files = files.filter((f) => f.startsWith(normalizedDir));
    }
    // Build directory tree (directories only, with file counts)
    const root = {
        name: directory || ".",
        path: directory || ".",
        type: "directory",
        children: [],
        file_count: 0,
    };
    // First, collect all unique directories and count files per directory
    const directoryCounts = new Map();
    const directorySet = new Set();
    for (const file of files) {
        const parts = file.split("/");
        // Track directories up to maxDepth
        for (let i = 0; i < parts.length - 1; i++) {
            const dirPath = parts.slice(0, i + 1).join("/");
            const depth = i + 1;
            if (depth <= maxDepth) {
                directorySet.add(dirPath);
            }
        }
        // Count files in their immediate parent directory (within maxDepth)
        if (parts.length > 1) {
            const parentDir = parts.slice(0, parts.length - 1).join("/");
            const parentDepth = parts.length - 1;
            if (parentDepth <= maxDepth) {
                directoryCounts.set(parentDir, (directoryCounts.get(parentDir) || 0) + 1);
            }
        }
        else {
            // File at root
            directoryCounts.set(".", (directoryCounts.get(".") || 0) + 1);
        }
    }
    // Build tree from directories
    const directories = Array.from(directorySet).sort();
    for (const dirPath of directories) {
        const parts = dirPath.split("/");
        let current = root;
        let currentPath = "";
        for (let i = 0; i < parts.length; i++) {
            const part = parts[i];
            if (!part)
                continue;
            currentPath = currentPath ? `${currentPath}/${part}` : part;
            if (!current.children) {
                current.children = [];
            }
            let child = current.children.find((c) => c.name === part);
            if (!child) {
                child = {
                    name: part,
                    path: currentPath,
                    type: "directory",
                    children: [],
                    file_count: directoryCounts.get(currentPath) || 0,
                };
                current.children.push(child);
            }
            current = child;
        }
    }
    // Set root file count
    root.file_count = directoryCounts.get(".") || 0;
    // If plain text format requested, render as tree
    if (formatAsPlainText) {
        const treeLines = renderTreeAsPlainText(root, showCounts, "", true);
        return {
            layout: treeLines.join("\n"),
            total_files: files.length,
            max_depth: maxDepth,
        }; // Type will be updated to support both formats
    }
    // Clean up file counts if not requested (JSON format only)
    if (!showCounts) {
        const removeFileCounts = (node) => {
            delete node.file_count;
            if (node.children) {
                node.children.forEach(removeFileCounts);
            }
        };
        removeFileCounts(root);
    }
    // Return JSON tree format
    return {
        root,
        total_files: files.length,
        max_depth: maxDepth,
    };
}
/**
 * Find files matching a pattern using both glob and fuzzy matching strategies.
 * This combines glob/micromatch for wildcard patterns with Levenshtein distance
 * for fuzzy matching to handle typos and partial matches.
 */
export async function findFile(files, pattern, opts) {
    const { directory, maxDistance = 3, maxResults = 50 } = opts ?? {};
    // Filter by directory if specified
    if (directory) {
        const normalizedDir = directory.endsWith("/") ? directory : directory + "/";
        files = files.filter((f) => f.startsWith(normalizedDir));
    }
    // Use the generic match function
    const matchResults = await match(files, (file) => file, pattern, {
        maxDistance,
        maxResults,
    });
    // Convert Match<string> to FileMatch by mapping data to path
    const fileMatches = matchResults.map((m) => ({
        path: m.data,
        score: m.score,
        match_type: m.match_type,
    }));
    // Add diagnostics when no matches found (only in evaluation mode)
    const enableDiagnostics = process.env.NUANCED_ENABLE_DIAGNOSTICS === "true";
    const diagnostic = enableDiagnostics && fileMatches.length === 0
        ? {
            searched_in: directory || "entire workspace",
            total_files_in_scope: files.length,
            pattern_type: "glob+fuzzy",
            suggestion: directory
                ? "Try broader pattern or remove --directory filter"
                : "Try increasing --max-distance or using exact filename",
            sample_files: files.slice(0, 5),
        }
        : undefined;
    return {
        matches: fileMatches,
        total_searched: files.length,
        pattern,
        ...(diagnostic && { diagnostic }),
    };
}
/**
 * Generic pattern matching function that combines glob and fuzzy matching strategies.
 * This function can match any type of data by extracting a string key for comparison.
 */
export async function match(values, key, pattern, opts) {
    const { maxDistance = 3, maxResults } = opts ?? {};
    const matches = [];
    const seenIndices = new Set();
    // Strategy 1: Glob/micromatch matching (for patterns like *.py, **/test_*.py, etc.)
    const micromatch = (await import("micromatch")).default;
    // Determine whether to match against basename or full path:
    // - If pattern contains "/" (like "django/db/*" or "**/test_*.py"), match against full path
    // - Otherwise (like "deletion.py" or "*delete*.py"), match against basename only
    const useBasename = !pattern.includes("/");
    values.forEach((value, index) => {
        const fullKey = key(value);
        const basename = fullKey.split("/").pop() || fullKey;
        const matchTarget = useBasename ? basename : fullKey;
        // Check if value matches the pattern
        if (micromatch.isMatch(matchTarget, pattern, { nocase: true })) {
            const isExact = basename.toLowerCase() ===
                pattern.replace(/[*?[\]]/g, "").toLowerCase() ||
                fullKey.toLowerCase() === pattern.replace(/[*?[\]]/g, "").toLowerCase();
            const isPrefix = basename
                .toLowerCase()
                .startsWith(pattern.replace(/[*?[\]]/g, "").toLowerCase()) ||
                fullKey
                    .toLowerCase()
                    .startsWith(pattern.replace(/[*?[\]]/g, "").toLowerCase());
            matches.push({
                data: value,
                score: isExact ? 1.0 : isPrefix ? 0.8 : 0.5,
                match_type: isExact ? "exact" : isPrefix ? "prefix" : "glob",
            });
            seenIndices.add(index);
        }
    });
    // Strategy 2: Fuzzy matching (for typos, partial matches, etc.)
    // This runs in addition to glob matching to catch cases where glob fails
    const { distance } = await import("fastest-levenshtein");
    // Remove glob wildcards for fuzzy matching
    const fuzzyPattern = pattern.replace(/[*?[\]]/g, "");
    // Only do fuzzy matching if pattern doesn't look like a pure glob (has wildcards but also text)
    if (fuzzyPattern.length > 0) {
        values.forEach((value, index) => {
            // Skip if already matched by glob
            if (seenIndices.has(index))
                return;
            const fullKey = key(value);
            const basename = fullKey.split("/").pop() || fullKey;
            const dist = distance(fuzzyPattern.toLowerCase(), basename.toLowerCase());
            if (dist <= maxDistance) {
                matches.push({
                    data: value,
                    score: 1 / (dist + 1), // Higher score for closer matches
                    match_type: dist === 0 ? "exact" : "fuzzy",
                });
            }
        });
    }
    // Sort by score (descending) and take top results
    matches.sort((a, b) => b.score - a.score);
    if (maxResults !== undefined) {
        return matches.slice(0, maxResults);
    }
    return matches;
}
