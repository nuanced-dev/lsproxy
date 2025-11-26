import { z } from "zod";
declare const __name__: unique symbol;
type Named<S extends z.ZodTypeAny, N extends string> = z.output<S> & {
    readonly [__name__]?: N;
};
export type Result<T, E> = Ok<T> | Err<E>;
export interface Err<E> {
    ok: false;
    data: E;
}
export interface Ok<T> {
    ok: true;
    data: T;
}
export declare const ok: <T>(data: T) => Ok<T>;
export declare const err: <E>(data: E) => Err<E>;
export declare const isOk: <T, E>(r: Result<T, E>) => r is Ok<T>;
export declare const isErr: <T, E>(r: Result<T, E>) => r is Err<E>;
export declare const DockerErrSchema: z.ZodObject<{
    error_code: z.ZodNumber;
    message: z.ZodString;
    stdout: z.ZodString;
    stderr: z.ZodString;
}, "strip", z.ZodTypeAny, {
    error_code: number;
    message: string;
    stdout: string;
    stderr: string;
}, {
    error_code: number;
    message: string;
    stdout: string;
    stderr: string;
}>;
export type DockerErr = Named<typeof DockerErrSchema, "DockerErr">;
export type DockerResult<T> = Result<T, DockerErr>;
export declare const DownResultSchema: z.ZodObject<{
    stdout: z.ZodString;
}, "strip", z.ZodTypeAny, {
    stdout: string;
}, {
    stdout: string;
}>;
export type DownResult = Named<typeof DownResultSchema, "DownResult">;
export declare const UpResultSchema: z.ZodObject<{
    host_port: z.ZodNumber;
    base_url: z.ZodString;
}, "strip", z.ZodTypeAny, {
    host_port: number;
    base_url: string;
}, {
    host_port: number;
    base_url: string;
}>;
export type UpResult = Named<typeof UpResultSchema, "UpResult">;
export declare const RunResultSchema: z.ZodObject<{
    stdout: z.ZodString;
}, "strip", z.ZodTypeAny, {
    stdout: string;
}, {
    stdout: string;
}>;
export type RunResult = Named<typeof RunResultSchema, "RunResult">;
export declare const StatusResultSchema: z.ZodObject<{
    container_name: z.ZodString;
    container_status: z.ZodString;
}, "strip", z.ZodTypeAny, {
    container_name: string;
    container_status: string;
}, {
    container_name: string;
    container_status: string;
}>;
export type StatusResult = Named<typeof StatusResultSchema, "StatusResult">;
export declare const LogsResultSchema: z.ZodObject<{
    stdout: z.ZodString;
}, "strip", z.ZodTypeAny, {
    stdout: string;
}, {
    stdout: string;
}>;
export type LogsResult = Named<typeof LogsResultSchema, "LogsResult">;
export declare const PullResultSchema: z.ZodObject<{
    image: z.ZodString;
    stdout: z.ZodString;
}, "strip", z.ZodTypeAny, {
    stdout: string;
    image: string;
}, {
    stdout: string;
    image: string;
}>;
export type PullResult = Named<typeof PullResultSchema, "PullResult">;
export declare const HttpErrSchema: z.ZodObject<{
    status_code: z.ZodNullable<z.ZodNumber>;
    error: z.ZodRecord<z.ZodString, z.ZodAny>;
}, "strip", z.ZodTypeAny, {
    status_code: number | null;
    error: Record<string, any>;
}, {
    status_code: number | null;
    error: Record<string, any>;
}>;
export type HttpErr = Named<typeof HttpErrSchema, "HttpErr">;
export type HttpResult<T> = Result<T, HttpErr>;
export declare const HealthResultSchema: z.ZodObject<{
    status: z.ZodUnion<[z.ZodLiteral<"ok">, z.ZodLiteral<"not ok">]>;
    version: z.ZodOptional<z.ZodString>;
    languages: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodBoolean>>;
}, "strip", z.ZodTypeAny, {
    status: "ok" | "not ok";
    version?: string | undefined;
    languages?: Record<string, boolean> | undefined;
}, {
    status: "ok" | "not ok";
    version?: string | undefined;
    languages?: Record<string, boolean> | undefined;
}>;
export type HealthResult = Named<typeof HealthResultSchema, "HealthResult">;
export declare const LspPositionSchema: z.ZodObject<{
    line: z.ZodNumber;
    character: z.ZodNumber;
}, "strip", z.ZodTypeAny, {
    line: number;
    character: number;
}, {
    line: number;
    character: number;
}>;
export type LspPosition = Named<typeof LspPositionSchema, "LspPosition">;
export declare const LspRangeSchema: z.ZodObject<{
    start: z.ZodObject<{
        line: z.ZodNumber;
        character: z.ZodNumber;
    }, "strip", z.ZodTypeAny, {
        line: number;
        character: number;
    }, {
        line: number;
        character: number;
    }>;
    end: z.ZodObject<{
        line: z.ZodNumber;
        character: z.ZodNumber;
    }, "strip", z.ZodTypeAny, {
        line: number;
        character: number;
    }, {
        line: number;
        character: number;
    }>;
}, "strip", z.ZodTypeAny, {
    start: {
        line: number;
        character: number;
    };
    end: {
        line: number;
        character: number;
    };
}, {
    start: {
        line: number;
        character: number;
    };
    end: {
        line: number;
        character: number;
    };
}>;
export type LspRange = Named<typeof LspRangeSchema, "LspRange">;
export declare const FilePositionSchema: z.ZodObject<{
    path: z.ZodString;
    position: z.ZodObject<{
        line: z.ZodNumber;
        character: z.ZodNumber;
    }, "strip", z.ZodTypeAny, {
        line: number;
        character: number;
    }, {
        line: number;
        character: number;
    }>;
}, "strip", z.ZodTypeAny, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}>;
export type FilePosition = Named<typeof FilePositionSchema, "FilePosition">;
export declare const FileRangeSchema: z.ZodObject<{
    path: z.ZodString;
    range: z.ZodObject<{
        start: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
        end: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    }, {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    }>;
}, "strip", z.ZodTypeAny, {
    path: string;
    range: {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    };
}, {
    path: string;
    range: {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    };
}>;
export type FileRange = Named<typeof FileRangeSchema, "FileRange">;
export declare const IdentifierPositionSchema: z.ZodObject<{
    path: z.ZodString;
    position: z.ZodObject<{
        line: z.ZodNumber;
        character: z.ZodNumber;
    }, "strip", z.ZodTypeAny, {
        line: number;
        character: number;
    }, {
        line: number;
        character: number;
    }>;
}, "strip", z.ZodTypeAny, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}>;
export type IdentifierPosition = Named<typeof IdentifierPositionSchema, "IdentifierPosition">;
export declare const SelectedIdentifierSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    kind: z.ZodNullable<z.ZodString>;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}>;
export type SelectedIdentifier = Named<typeof SelectedIdentifierSchema, "SelectedIdentifier">;
export declare const ListFilesResultSchema: z.ZodArray<z.ZodString, "many">;
export type ListFilesResult = Named<typeof ListFilesResultSchema, "ListFilesResult">;
export declare const ReadSourceResultSchema: z.ZodObject<{
    source_code: z.ZodString;
}, "strip", z.ZodTypeAny, {
    source_code: string;
}, {
    source_code: string;
}>;
export type ReadSourceResult = Named<typeof ReadSourceResultSchema, "ReadSourceResult">;
export declare const DefinitionLocationSchema: z.ZodObject<{
    path: z.ZodString;
    position: z.ZodObject<{
        line: z.ZodNumber;
        character: z.ZodNumber;
    }, "strip", z.ZodTypeAny, {
        line: number;
        character: number;
    }, {
        line: number;
        character: number;
    }>;
}, "strip", z.ZodTypeAny, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}>;
export type DefinitionLocation = Named<typeof DefinitionLocationSchema, "DefinitionLocation">;
export declare const ContextSnippetSchema: z.ZodEffects<z.ZodObject<{
    file_range: z.ZodOptional<z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>>;
    range: z.ZodOptional<z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>>;
    source_code: z.ZodString;
}, "strip", z.ZodTypeAny, {
    source_code: string;
    range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
    file_range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
}, {
    source_code: string;
    range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
    file_range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
}>, {
    source_code: string;
    range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
    file_range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
}, {
    source_code: string;
    range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
    file_range?: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    } | undefined;
}>;
export type ContextSnippet = Named<typeof ContextSnippetSchema, "ContextSnippet">;
export declare const DefinitionInFileSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    identifier_position: z.ZodObject<{
        path: z.ZodString;
        position: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }>;
    kind: z.ZodString;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string;
    name: string;
    identifier_position: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    };
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string;
    name: string;
    identifier_position: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    };
}>;
export type DefinitionInFile = Named<typeof DefinitionInFileSchema, "DefinitionInFile">;
export declare const DefinitionsInFileResultSchema: z.ZodArray<z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    identifier_position: z.ZodObject<{
        path: z.ZodString;
        position: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }>;
    kind: z.ZodString;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string;
    name: string;
    identifier_position: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    };
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string;
    name: string;
    identifier_position: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    };
}>, "many">;
export type DefinitionsInFileResult = Named<typeof DefinitionsInFileResultSchema, "DefinitionsInFileResult">;
export declare const FindDefinitionResultSchema: z.ZodObject<{
    definitions: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        position: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }>, "many">;
    selected_identifier: z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }>;
    raw_response: z.ZodUnknown;
    source_code_context: z.ZodNullable<z.ZodArray<z.ZodEffects<z.ZodObject<{
        file_range: z.ZodOptional<z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>>;
        range: z.ZodOptional<z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>>;
        source_code: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }>, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }>, "many">>;
}, "strip", z.ZodTypeAny, {
    definitions: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }[];
    selected_identifier: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    };
    source_code_context: {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }[] | null;
    raw_response?: unknown;
}, {
    definitions: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }[];
    selected_identifier: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    };
    source_code_context: {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }[] | null;
    raw_response?: unknown;
}>;
export type FindDefinitionResult = Named<typeof FindDefinitionResultSchema, "FindDefinitionResult">;
export declare const IdentifierSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    kind: z.ZodNullable<z.ZodString>;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}>;
export type Identifier = Named<typeof IdentifierSchema, "Identifier">;
export declare const FindIdentifierResultSchema: z.ZodObject<{
    identifiers: z.ZodArray<z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    identifiers: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }[];
}, {
    identifiers: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }[];
}>;
export type FindIdentifierResult = Named<typeof FindIdentifierResultSchema, "FindIdentifierResult">;
export declare const ExternalSymbolSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    kind: z.ZodNullable<z.ZodString>;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}>;
export type ExternalSymbol = Named<typeof ExternalSymbolSchema, "ExternalSymbol">;
export declare const NotFoundSymbolSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    kind: z.ZodNullable<z.ZodString>;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}>;
export type NotFoundSymbol = Named<typeof NotFoundSymbolSchema, "NotFoundSymbol">;
export declare const WorkspaceDefinitionSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    identifier_position: z.ZodObject<{
        path: z.ZodString;
        position: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }>;
    kind: z.ZodNullable<z.ZodString>;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
    identifier_position: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    };
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
    identifier_position: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    };
}>;
export type WorkspaceDefinition = Named<typeof WorkspaceDefinitionSchema, "WorkspaceDefinition">;
export declare const WorkspaceReferenceSchema: z.ZodObject<{
    file_range: z.ZodObject<{
        path: z.ZodString;
        range: z.ZodObject<{
            start: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
            end: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }, {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }, {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    }>;
    kind: z.ZodNullable<z.ZodString>;
    name: z.ZodString;
}, "strip", z.ZodTypeAny, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}, {
    file_range: {
        path: string;
        range: {
            start: {
                line: number;
                character: number;
            };
            end: {
                line: number;
                character: number;
            };
        };
    };
    kind: string | null;
    name: string;
}>;
export type WorkspaceReference = Named<typeof WorkspaceReferenceSchema, "WorkspaceReference">;
export declare const WorkspaceSymbolSchema: z.ZodObject<{
    reference: z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }>;
    definitions: z.ZodArray<z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        identifier_position: z.ZodObject<{
            path: z.ZodString;
            position: z.ZodObject<{
                line: z.ZodNumber;
                character: z.ZodNumber;
            }, "strip", z.ZodTypeAny, {
                line: number;
                character: number;
            }, {
                line: number;
                character: number;
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            position: {
                line: number;
                character: number;
            };
        }, {
            path: string;
            position: {
                line: number;
                character: number;
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
        identifier_position: {
            path: string;
            position: {
                line: number;
                character: number;
            };
        };
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
        identifier_position: {
            path: string;
            position: {
                line: number;
                character: number;
            };
        };
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    definitions: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
        identifier_position: {
            path: string;
            position: {
                line: number;
                character: number;
            };
        };
    }[];
    reference: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    };
}, {
    definitions: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
        identifier_position: {
            path: string;
            position: {
                line: number;
                character: number;
            };
        };
    }[];
    reference: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    };
}>;
export type WorkspaceSymbol = Named<typeof WorkspaceSymbolSchema, "WorkspaceSymbol">;
export declare const FindReferencedSymbolsResultSchema: z.ZodObject<{
    external_symbols: z.ZodArray<z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }>, "many">;
    not_found: z.ZodArray<z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }>, "many">;
    workspace_symbols: z.ZodArray<z.ZodObject<{
        reference: z.ZodObject<{
            file_range: z.ZodObject<{
                path: z.ZodString;
                range: z.ZodObject<{
                    start: z.ZodObject<{
                        line: z.ZodNumber;
                        character: z.ZodNumber;
                    }, "strip", z.ZodTypeAny, {
                        line: number;
                        character: number;
                    }, {
                        line: number;
                        character: number;
                    }>;
                    end: z.ZodObject<{
                        line: z.ZodNumber;
                        character: z.ZodNumber;
                    }, "strip", z.ZodTypeAny, {
                        line: number;
                        character: number;
                    }, {
                        line: number;
                        character: number;
                    }>;
                }, "strip", z.ZodTypeAny, {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                }, {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                }>;
            }, "strip", z.ZodTypeAny, {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            }, {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            }>;
            kind: z.ZodNullable<z.ZodString>;
            name: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
        }, {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
        }>;
        definitions: z.ZodArray<z.ZodObject<{
            file_range: z.ZodObject<{
                path: z.ZodString;
                range: z.ZodObject<{
                    start: z.ZodObject<{
                        line: z.ZodNumber;
                        character: z.ZodNumber;
                    }, "strip", z.ZodTypeAny, {
                        line: number;
                        character: number;
                    }, {
                        line: number;
                        character: number;
                    }>;
                    end: z.ZodObject<{
                        line: z.ZodNumber;
                        character: z.ZodNumber;
                    }, "strip", z.ZodTypeAny, {
                        line: number;
                        character: number;
                    }, {
                        line: number;
                        character: number;
                    }>;
                }, "strip", z.ZodTypeAny, {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                }, {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                }>;
            }, "strip", z.ZodTypeAny, {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            }, {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            }>;
            identifier_position: z.ZodObject<{
                path: z.ZodString;
                position: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            }, {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            }>;
            kind: z.ZodNullable<z.ZodString>;
            name: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
            identifier_position: {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            };
        }, {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
            identifier_position: {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            };
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        definitions: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
            identifier_position: {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            };
        }[];
        reference: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
        };
    }, {
        definitions: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
            identifier_position: {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            };
        }[];
        reference: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
        };
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    external_symbols: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }[];
    not_found: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }[];
    workspace_symbols: {
        definitions: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
            identifier_position: {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            };
        }[];
        reference: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
        };
    }[];
}, {
    external_symbols: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }[];
    not_found: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }[];
    workspace_symbols: {
        definitions: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
            identifier_position: {
                path: string;
                position: {
                    line: number;
                    character: number;
                };
            };
        }[];
        reference: {
            file_range: {
                path: string;
                range: {
                    start: {
                        line: number;
                        character: number;
                    };
                    end: {
                        line: number;
                        character: number;
                    };
                };
            };
            kind: string | null;
            name: string;
        };
    }[];
}>;
export type FindReferencedSymbolsResult = Named<typeof FindReferencedSymbolsResultSchema, "FindReferencedSymbolsResult">;
export declare const ReferenceLocationSchema: z.ZodObject<{
    path: z.ZodString;
    position: z.ZodObject<{
        line: z.ZodNumber;
        character: z.ZodNumber;
    }, "strip", z.ZodTypeAny, {
        line: number;
        character: number;
    }, {
        line: number;
        character: number;
    }>;
}, "strip", z.ZodTypeAny, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}, {
    path: string;
    position: {
        line: number;
        character: number;
    };
}>;
export type ReferenceLocation = Named<typeof ReferenceLocationSchema, "ReferenceLocation">;
export declare const FindReferencesResultSchema: z.ZodObject<{
    references: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        position: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }, {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }>, "many">;
    selected_identifier: z.ZodObject<{
        file_range: z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>;
        kind: z.ZodNullable<z.ZodString>;
        name: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }, {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    }>;
    context: z.ZodNullable<z.ZodArray<z.ZodEffects<z.ZodObject<{
        file_range: z.ZodOptional<z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>>;
        range: z.ZodOptional<z.ZodObject<{
            path: z.ZodString;
            range: z.ZodObject<{
                start: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
                end: z.ZodObject<{
                    line: z.ZodNumber;
                    character: z.ZodNumber;
                }, "strip", z.ZodTypeAny, {
                    line: number;
                    character: number;
                }, {
                    line: number;
                    character: number;
                }>;
            }, "strip", z.ZodTypeAny, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }, {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            }>;
        }, "strip", z.ZodTypeAny, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }, {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }>>;
        source_code: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }>, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }, {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }>, "many">>;
    raw_response: z.ZodUnknown;
}, "strip", z.ZodTypeAny, {
    selected_identifier: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    };
    references: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }[];
    context: {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }[] | null;
    raw_response?: unknown;
}, {
    selected_identifier: {
        file_range: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        };
        kind: string | null;
        name: string;
    };
    references: {
        path: string;
        position: {
            line: number;
            character: number;
        };
    }[];
    context: {
        source_code: string;
        range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
        file_range?: {
            path: string;
            range: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        } | undefined;
    }[] | null;
    raw_response?: unknown;
}>;
export type FindReferencesResult = Named<typeof FindReferencesResultSchema, "FindReferencesResult">;
export declare const BaseCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
}, {
    json?: boolean | undefined;
}>;
export type BaseCommandOptions = Named<typeof BaseCommandOptionsSchema, "BaseCommandOptions">;
export declare const UpCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    containerName: z.ZodOptional<z.ZodString>;
    hostPort: z.ZodOptional<z.ZodNumber>;
    languageContainerVersion: z.ZodOptional<z.ZodString>;
    proxyImage: z.ZodOptional<z.ZodString>;
    wrapperImage: z.ZodOptional<z.ZodString>;
    watchdogImage: z.ZodOptional<z.ZodString>;
    timeout: z.ZodOptional<z.ZodNumber>;
    sudo: z.ZodOptional<z.ZodBoolean>;
    stream: z.ZodOptional<z.ZodBoolean>;
    ro: z.ZodOptional<z.ZodBoolean>;
    bindHost: z.ZodOptional<z.ZodString>;
    debug: z.ZodOptional<z.ZodBoolean>;
    env: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    envFile: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    hostPort?: number | undefined;
    languageContainerVersion?: string | undefined;
    proxyImage?: string | undefined;
    wrapperImage?: string | undefined;
    watchdogImage?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
    ro?: boolean | undefined;
    bindHost?: string | undefined;
    debug?: boolean | undefined;
    env?: string[] | undefined;
    envFile?: string | undefined;
}, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    hostPort?: number | undefined;
    languageContainerVersion?: string | undefined;
    proxyImage?: string | undefined;
    wrapperImage?: string | undefined;
    watchdogImage?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
    ro?: boolean | undefined;
    bindHost?: string | undefined;
    debug?: boolean | undefined;
    env?: string[] | undefined;
    envFile?: string | undefined;
}>;
export type UpCommandOptions = Named<typeof UpCommandOptionsSchema, "UpCommandOptions">;
export declare const DownCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    containerName: z.ZodOptional<z.ZodString>;
    sudo: z.ZodOptional<z.ZodBoolean>;
    timeout: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
}, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
}>;
export type DownCommandOptions = Named<typeof DownCommandOptionsSchema, "DownCommandOptions">;
export declare const LogsCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    containerName: z.ZodOptional<z.ZodString>;
    timeout: z.ZodOptional<z.ZodNumber>;
    sudo: z.ZodOptional<z.ZodBoolean>;
    stream: z.ZodOptional<z.ZodBoolean>;
    since: z.ZodOptional<z.ZodString>;
    tail: z.ZodOptional<z.ZodUnion<[z.ZodNumber, z.ZodLiteral<"all">]>>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
    since?: string | undefined;
    tail?: number | "all" | undefined;
}, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
    since?: string | undefined;
    tail?: number | "all" | undefined;
}>;
export type LogsCommandOptions = Named<typeof LogsCommandOptionsSchema, "LogsCommandOptions">;
export declare const RunCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    containerName: z.ZodOptional<z.ZodString>;
    sudo: z.ZodOptional<z.ZodBoolean>;
    stream: z.ZodOptional<z.ZodBoolean>;
    timeout: z.ZodOptional<z.ZodNumber>;
    env: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    envFile: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
    env?: string[] | undefined;
    envFile?: string | undefined;
}, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
    env?: string[] | undefined;
    envFile?: string | undefined;
}>;
export type RunCommandOptions = Named<typeof RunCommandOptionsSchema, "RunCommandOptions">;
export declare const StatusCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    containerName: z.ZodOptional<z.ZodString>;
    sudo: z.ZodOptional<z.ZodBoolean>;
    timeout: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
}, {
    json?: boolean | undefined;
    containerName?: string | undefined;
    timeout?: number | undefined;
    sudo?: boolean | undefined;
}>;
export type StatusCommandOptions = Named<typeof StatusCommandOptionsSchema, "StatusCommandOptions">;
export declare const PullCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    image: z.ZodOptional<z.ZodString>;
    stream: z.ZodOptional<z.ZodBoolean>;
    sudo: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    image?: string | undefined;
    json?: boolean | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
}, {
    image?: string | undefined;
    json?: boolean | undefined;
    sudo?: boolean | undefined;
    stream?: boolean | undefined;
}>;
export type PullCommandOptions = Named<typeof PullCommandOptionsSchema, "PullCommandOptions">;
export declare const LspCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}>;
export type LspCommandOptions = Named<typeof LspCommandOptionsSchema, "LspCommandOptions">;
export declare const FindDefinitionOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
} & {
    includeRawResponse: z.ZodOptional<z.ZodBoolean>;
    includeSourceCode: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
    includeRawResponse?: boolean | undefined;
    includeSourceCode?: boolean | undefined;
}, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
    includeRawResponse?: boolean | undefined;
    includeSourceCode?: boolean | undefined;
}>;
export type FindDefinitionOptions = Named<typeof FindDefinitionOptionsSchema, "FindDefinitionOptions">;
export declare const ReadSourceCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
} & {
    range: z.ZodOptional<z.ZodNullable<z.ZodLazy<z.ZodObject<{
        start: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
        end: z.ZodObject<{
            line: z.ZodNumber;
            character: z.ZodNumber;
        }, "strip", z.ZodTypeAny, {
            line: number;
            character: number;
        }, {
            line: number;
            character: number;
        }>;
    }, "strip", z.ZodTypeAny, {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    }, {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    }>>>>;
}, "strip", z.ZodTypeAny, {
    range?: {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    } | null | undefined;
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}, {
    range?: {
        start: {
            line: number;
            character: number;
        };
        end: {
            line: number;
            character: number;
        };
    } | null | undefined;
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}>;
export type ReadSourceCommandOptions = Named<typeof ReadSourceCommandOptionsSchema, "ReadSourceCommandOptions">;
export declare const HealthCommandOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}>;
export type HealthCommandOptions = Named<typeof HealthCommandOptionsSchema, "HealthCommandOptions">;
export declare const FindIdentifierOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
} & {
    position: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    position?: string | undefined;
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}, {
    position?: string | undefined;
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
}>;
export type FindIdentifierOptions = Named<typeof FindIdentifierOptionsSchema, "FindIdentifierOptions">;
export declare const FindReferencedSymbolsOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
} & {
    fullScan: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
    fullScan?: boolean | undefined;
}, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
    fullScan?: boolean | undefined;
}>;
export type FindReferencedSymbolsOptions = Named<typeof FindReferencedSymbolsOptionsSchema, "FindReferencedSymbolsOptions">;
export declare const FindReferencesOptionsSchema: z.ZodObject<{
    json: z.ZodOptional<z.ZodBoolean>;
} & {
    lspUrl: z.ZodOptional<z.ZodString>;
    lspPort: z.ZodOptional<z.ZodNumber>;
    timeout: z.ZodOptional<z.ZodNumber>;
} & {
    includeCodeContextLines: z.ZodOptional<z.ZodNumber>;
    includeRawResponse: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
    includeRawResponse?: boolean | undefined;
    includeCodeContextLines?: number | undefined;
}, {
    json?: boolean | undefined;
    timeout?: number | undefined;
    lspUrl?: string | undefined;
    lspPort?: number | undefined;
    includeRawResponse?: boolean | undefined;
    includeCodeContextLines?: number | undefined;
}>;
export type FindReferencesOptions = Named<typeof FindReferencesOptionsSchema, "FindReferencesOptions">;
export {};
//# sourceMappingURL=types.d.ts.map