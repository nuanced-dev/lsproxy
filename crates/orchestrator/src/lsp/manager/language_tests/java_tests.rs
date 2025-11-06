use super::*;
use crate::api_types::{Position, Range as ApiRange};
use lsp_types::{Position as LspPosition, Range as LspRange};

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&java_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "AStar.java";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("AStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 10,
                    character: 13,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.java"),
                range: ApiRange {
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 96,
                        character: 21,
                    },
                },
            },
        },
        Symbol {
            name: String::from("findPathTo"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 39,
                    character: 22,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.java"),
                range: ApiRange {
                    start: Position {
                        line: 39,
                        character: 0,
                    },
                    end: Position {
                        line: 59,
                        character: 5,
                    },
                },
            },
        },
        Symbol {
            name: String::from("addNeigborsToOpenList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 61,
                    character: 17,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.java"),
                range: ApiRange {
                    start: Position {
                        line: 61,
                        character: 0,
                    },
                    end: Position {
                        line: 89,
                        character: 41,
                    },
                },
            },
        },
        Symbol {
            name: String::from("distance"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 93,
                    character: 55,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.java"),
                range: ApiRange {
                    start: Position {
                        line: 93,
                        character: 0,
                    },
                    end: Position {
                        line: 95,
                        character: 41,
                    },
                },
            },
        },
        Symbol {
            name: String::from("main"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 98,
                    character: 59,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.java"),
                range: ApiRange {
                    start: Position {
                        line: 98,
                        character: 0,
                    },
                    end: Position {
                        line: 136,
                        character: 5,
                    },
                },
            },
        },
        Symbol {
            name: String::from("findNeighborInList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 138,
                    character: 20,
                },
            },
            file_range: FileRange {
                path: String::from("AStar.java"),
                range: ApiRange {
                    start: Position {
                        line: 138,
                        character: 0,
                    },
                    end: Position {
                        line: 140,
                        character: 5,
                    },
                },
            },
        },
    ];

    // sort symbols by name
    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());
    assert_eq!(symbol_response, expected);
    Ok(())
}

#[tokio::test]
async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&java_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "AStar.java";
    let references = manager
        .find_references(
            file_path,
            LspPosition {
                line: 10,
                character: 13,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
            range: LspRange {
                start: LspPosition {
                    line: 10,
                    character: 13,
                },
                end: LspPosition {
                    line: 10,
                    character: 18,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
            range: LspRange {
                start: LspPosition {
                    line: 111,
                    character: 8,
                },
                end: LspPosition {
                    line: 111,
                    character: 13,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
            range: LspRange {
                start: LspPosition {
                    line: 111,
                    character: 23,
                },
                end: LspPosition {
                    line: 111,
                    character: 28,
                },
            },
        },
    ];
    assert_eq!(references, expected);
    Ok(())
}

#[tokio::test]
async fn test_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&java_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let definition_response = manager
        .find_definition(
            "AStar.java",
            LspPosition {
                line: 111,
                character: 8,
            },
        )
        .await?;

    let definitions = match definition_response {
        GotoDefinitionResponse::Scalar(location) => vec![location],
        GotoDefinitionResponse::Array(locations) => locations,
        GotoDefinitionResponse::Link(_links) => Vec::new(),
    };
    let expected = vec![Location {
        uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
        range: LspRange {
            start: LspPosition {
                line: 10,
                character: 13,
            },
            end: LspPosition {
                line: 10,
                character: 18,
            },
        },
    }];

    assert_eq!(definitions, expected);
    Ok(())
}
