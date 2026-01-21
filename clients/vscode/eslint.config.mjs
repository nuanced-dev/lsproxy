import { fileURLToPath } from "url";
import { dirname } from "path";
import js from "@eslint/js";
import typescript from "typescript-eslint";
import prettierConfigRecommended from "eslint-plugin-prettier/recommended";
import importPlugin from "eslint-plugin-import";
import stylistic from "@stylistic/eslint-plugin";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default [
  js.configs.recommended,
  ...typescript.configs.recommended,
  prettierConfigRecommended,
  importPlugin.flatConfigs.recommended,
  importPlugin.flatConfigs.typescript,
  {
    files: ["**/*.ts", "**/*.tsx"],
    settings: {
      "import/resolver": {
        typescript: true,
        node: true,
      },
    },
    languageOptions: {
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "commonjs",
        tsconfigRootDir: __dirname,
      },
      globals: {
        clearTimeout: "readonly",
        fetch: "readonly",
        global: "readonly",
        setTimeout: "readonly",
        Response: "readonly",
        __dirname: "readonly",
        __filename: "readonly",
        AbortController: "readonly",
        Buffer: "readonly",
        BufferEncoding: "readonly",
        NodeJS: "readonly",
      },
    },
    plugins: {
      "@stylistic": stylistic,
    },
    rules: {
      "no-console": "off",
      "@typescript-eslint/explicit-function-return-type": "off",
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          destructuredArrayIgnorePattern: "^_",
        },
      ],
      "prettier/prettier": "error",
      "@stylistic/comma-dangle": ["error", "always-multiline"],
      "@stylistic/eol-last": ["error", "always"],
      "@stylistic/no-multiple-empty-lines": ["error", { max: 1, maxEOF: 0 }],
      "@stylistic/no-trailing-spaces": "error",
      "@stylistic/quotes": ["error", "double"],
      "@stylistic/semi": ["error", "always"],
      "import/first": "error",
      "import/no-extraneous-dependencies": "error",
      "import/no-unassigned-import": "error",
      "import/no-unresolved": "error",
      "import/order": "error",
    },
  },
  {
    ignores: ["*.config.cjs", "*.config.mjs", "dist/", "node_modules/"],
  },
];
