import type { Monaco } from "@monaco-editor/react";

import type { CompilationError } from "../types/policy";

const monarchLanguage = {
  defaultToken: "invalid",
  keywords: ["allow", "deny", "if", "and", "or", "not", "in"],
  tokenizer: {
    root: [
      [/#.*$/, "comment"],
      [
        /\b(?:allow|deny|if|and|or|not|in)\b/,
        "keyword",
      ],
      [
        /\b(?:subject|resource|environment|action)\b/,
        "type.identifier",
      ],
      [/==|!=|<=|>=|<|>/, "operator"],
      [/"/, { token: "string.quote", next: "@string" }],
      [/\d+\.\d+/, "number.float"],
      [/\d+/, "number"],
      [/[a-zA-Z_][\w.]*/, "identifier"],
      [/[\[\](){}:,]/, "delimiter"],
      [/[ \t\r\n]+/, "white"],
    ],
    string: [
      [/[^"\\]+/, "string"],
      [/\\./, "string.escape"],
      [/"/, { token: "string.quote", next: "@pop" }],
    ],
  },
} satisfies Monaco.languages.IMonarchLanguage;

const languageConfiguration = {
  comments: {
    lineComment: "#",
  },
  brackets: [
    ["[", "]"],
    ["(", ")"],
  ],
  autoClosingPairs: [
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
  ],
  surroundingPairs: [
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
  ],
} satisfies Monaco.languages.LanguageConfiguration;

export function registerPolicyDslLanguage(monaco: Monaco) {
  const alreadyRegistered = monaco.languages
    .getLanguages()
    .some((language) => language.id === "policy-dsl");

  if (!alreadyRegistered) {
    monaco.languages.register({ id: "policy-dsl" });
    monaco.languages.setMonarchTokensProvider(
      "policy-dsl",
      monarchLanguage,
    );
    monaco.languages.setLanguageConfiguration(
      "policy-dsl",
      languageConfiguration,
    );
  }
}

export function createErrorMarkers(
  monaco: Monaco,
  errors: CompilationError[],
) {
  return errors.map((error) => {
    const line = error.line ?? 1;
    const column = error.column ?? 1;

    return {
      severity: monaco.MarkerSeverity.Error,
      startLineNumber: line,
      startColumn: column,
      endLineNumber: line,
      endColumn: column + 100,
      message: error.message,
    } satisfies monaco.editor.IMarkerData;
  });
}
