// Constants for services and languages

export const ALL_SERVICES = ["proxy", "watchdog", "wrapper"] as const;

const UNVERSIONED_LANGUAGES = [
  "clangd",
  "csharp",
  "golang",
  "java",
  "php",
  "python",
  "rust",
  "typescript",
] as const;

const SUPPORTED_RUBY_VERSIONS = [
  "3.2.2",
  "3.2.6",
  "3.3.5",
  "3.3.6",
  "3.3.7",
  "3.3.8",
  "3.3.9",
  "3.3.10",
  "3.4.0",
  "3.4.1",
  "3.4.2",
  "3.4.3",
  "3.4.4",
  "3.4.5",
  "3.4.6",
  "3.4.7",
] as const;

export const ALL_LANGUAGES = [
  ...UNVERSIONED_LANGUAGES,
  ...SUPPORTED_RUBY_VERSIONS.map((v) => `ruby-${v}`),
  ...SUPPORTED_RUBY_VERSIONS.map((v) => `ruby-sorbet-${v}`),
];

export function selectLanguages(filter: string[], all: string[]): string[] {
  const languages: string[] = [];
  for (const name of filter) {
    const exactMatches = all.filter((v) => v === name);
    languages.push(...exactMatches);
    const versionedMatches = all.filter((v) => {
      if (!v.startsWith(`${name}-`)) {
        return false;
      }
      const afterPrefix = v.slice(name.length + 1);
      return /^\d/.test(afterPrefix);
    });
    languages.push(...versionedMatches);
  }
  return languages;
}
