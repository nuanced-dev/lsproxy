use super::*;
use crate::api_types::{FilePosition, FileRange, Position, Range, Symbol, SymbolResponse};

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&csharp_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "AStar.cs";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let expected = vec![
        Symbol {
            name: String::from("AStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 2,
                    character: 17,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 87,
                        character: 5,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_open"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 4,
                    character: 36,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 50,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_closed"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 5,
                    character: 36,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 52,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_path"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 6,
                    character: 36,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 6,
                        character: 0,
                    },
                    end: Position {
                        line: 6,
                        character: 50,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_maze"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 7,
                    character: 33,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 7,
                        character: 0,
                    },
                    end: Position {
                        line: 7,
                        character: 39,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_current"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 8,
                    character: 21,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 8,
                        character: 0,
                    },
                    end: Position {
                        line: 8,
                        character: 30,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_xStart"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 9,
                    character: 29,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 9,
                        character: 0,
                    },
                    end: Position {
                        line: 9,
                        character: 37,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_yStart"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 10,
                    character: 29,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 10,
                        character: 37,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_xEnd"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 11,
                    character: 20,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 11,
                        character: 0,
                    },
                    end: Position {
                        line: 11,
                        character: 33,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_yEnd"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 11,
                    character: 27,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 11,
                        character: 0,
                    },
                    end: Position {
                        line: 11,
                        character: 33,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_diag"),
            kind: String::from("field"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 12,
                    character: 30,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 12,
                        character: 0,
                    },
                    end: Position {
                        line: 12,
                        character: 36,
                    },
                },
            },
        },
        Symbol {
            name: String::from("maze"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 14,
                    character: 29,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 14,
                        character: 0,
                    },
                    end: Position {
                        line: 14,
                        character: 33,
                    },
                },
            },
        },
        Symbol {
            name: String::from("xStart"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 14,
                    character: 39,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 14,
                        character: 0,
                    },
                    end: Position {
                        line: 14,
                        character: 45,
                    },
                },
            },
        },
        Symbol {
            name: String::from("yStart"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 14,
                    character: 51,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 14,
                        character: 0,
                    },
                    end: Position {
                        line: 14,
                        character: 57,
                    },
                },
            },
        },
        Symbol {
            name: String::from("diag"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 14,
                    character: 64,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 14,
                        character: 0,
                    },
                    end: Position {
                        line: 14,
                        character: 68,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_maze"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 16,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 16,
                        character: 0,
                    },
                    end: Position {
                        line: 16,
                        character: 17,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_current"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 17,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 17,
                        character: 0,
                    },
                    end: Position {
                        line: 17,
                        character: 20,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_xStart"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 18,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 18,
                        character: 0,
                    },
                    end: Position {
                        line: 18,
                        character: 19,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_yStart"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 19,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 19,
                        character: 0,
                    },
                    end: Position {
                        line: 19,
                        character: 19,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_diag"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 20,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 20,
                        character: 0,
                    },
                    end: Position {
                        line: 20,
                        character: 17,
                    },
                },
            },
        },
        Symbol {
            name: String::from("FindPathTo"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 23,
                    character: 27,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 23,
                        character: 0,
                    },
                    end: Position {
                        line: 49,
                        character: 9,
                    },
                },
            },
        },
        Symbol {
            name: String::from("xEnd"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 23,
                    character: 42,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 23,
                        character: 0,
                    },
                    end: Position {
                        line: 23,
                        character: 46,
                    },
                },
            },
        },
        Symbol {
            name: String::from("yEnd"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 23,
                    character: 52,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 23,
                        character: 0,
                    },
                    end: Position {
                        line: 23,
                        character: 56,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_xEnd"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 25,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 25,
                        character: 0,
                    },
                    end: Position {
                        line: 25,
                        character: 17,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_yEnd"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 26,
                    character: 12,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 26,
                        character: 0,
                    },
                    end: Position {
                        line: 26,
                        character: 17,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_current"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 35,
                    character: 16,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 35,
                        character: 0,
                    },
                    end: Position {
                        line: 35,
                        character: 24,
                    },
                },
            },
        },
        Symbol {
            name: String::from("_current"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 44,
                    character: 16,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 44,
                        character: 0,
                    },
                    end: Position {
                        line: 44,
                        character: 24,
                    },
                },
            },
        },
        Symbol {
            name: String::from("AddNeighborsToOpenList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 51,
                    character: 21,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 51,
                        character: 0,
                    },
                    end: Position {
                        line: 76,
                        character: 9,
                    },
                },
            },
        },
        Symbol {
            name: String::from("x"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 53,
                    character: 21,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 53,
                        character: 0,
                    },
                    end: Position {
                        line: 53,
                        character: 22,
                    },
                },
            },
        },
        Symbol {
            name: String::from("y"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 55,
                    character: 25,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 55,
                        character: 0,
                    },
                    end: Position {
                        line: 55,
                        character: 26,
                    },
                },
            },
        },
        Symbol {
            name: String::from("node"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 60,
                    character: 24,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 60,
                        character: 0,
                    },
                    end: Position {
                        line: 60,
                        character: 28,
                    },
                },
            },
        },
        Symbol {
            name: String::from("Distance"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 78,
                    character: 23,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 78,
                        character: 0,
                    },
                    end: Position {
                        line: 81,
                        character: 9,
                    },
                },
            },
        },
        Symbol {
            name: String::from("x"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 78,
                    character: 36,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 78,
                        character: 0,
                    },
                    end: Position {
                        line: 78,
                        character: 37,
                    },
                },
            },
        },
        Symbol {
            name: String::from("y"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 78,
                    character: 43,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 78,
                        character: 0,
                    },
                    end: Position {
                        line: 78,
                        character: 44,
                    },
                },
            },
        },
        Symbol {
            name: String::from("FindNeighborInList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 83,
                    character: 21,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 83,
                        character: 0,
                    },
                    end: Position {
                        line: 86,
                        character: 9,
                    },
                },
            },
        },
        Symbol {
            name: String::from("list"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 83,
                    character: 51,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 83,
                        character: 0,
                    },
                    end: Position {
                        line: 83,
                        character: 55,
                    },
                },
            },
        },
        Symbol {
            name: String::from("node"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("AStar.cs"),
                position: Position {
                    line: 83,
                    character: 62,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.cs"),
                range: Range {
                    start: Position {
                        line: 83,
                        character: 0,
                    },
                    end: Position {
                        line: 83,
                        character: 66,
                    },
                },
            },
        },
    ];
    // Sort definitions
    let mut sorted_response = symbol_response;
    sorted_response.sort_by(|a, b| {
        let path_cmp = a.identifier_position.path.cmp(&b.identifier_position.path);
        if path_cmp.is_eq() {
            let line_cmp = a
                .identifier_position
                .position
                .line
                .cmp(&b.identifier_position.position.line);
            if line_cmp.is_eq() {
                a.identifier_position
                    .position
                    .character
                    .cmp(&b.identifier_position.position.character)
            } else {
                line_cmp
            }
        } else {
            path_cmp
        }
    });

    let mut sorted_expected = expected;
    sorted_expected.sort_by(|a, b| {
        let path_cmp = a.identifier_position.path.cmp(&b.identifier_position.path);
        if path_cmp.is_eq() {
            let line_cmp = a
                .identifier_position
                .position
                .line
                .cmp(&b.identifier_position.position.line);
            if line_cmp.is_eq() {
                a.identifier_position
                    .position
                    .character
                    .cmp(&b.identifier_position.position.character)
            } else {
                line_cmp
            }
        } else {
            path_cmp
        }
    });

    assert_eq!(sorted_response, sorted_expected);
    Ok(())
}
