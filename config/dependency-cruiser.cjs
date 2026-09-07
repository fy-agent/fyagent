/** Runtime ownership; executed from the repository root with an explicit config path. */
module.exports = {
  forbidden: [
    {
      name: "no-runtime-cycle",
      severity: "error",
      from: {},
      to: { circular: true },
    },
    {
      name: "no-unresolved-runtime-import",
      severity: "error",
      from: {},
      to: { couldNotResolve: true },
    },
    {
      name: "domain-does-not-import-renderer-or-native-runtime",
      severity: "error",
      from: { path: "^src/domain/" },
      to: {
        path: "^src/(?!domain/)|(?:^|/)node_modules/(?:react(?:-dom)?|@tauri-apps)(?:/|$)",
      },
    },
    {
      name: "shared-does-not-import-pages-widgets-or-app",
      severity: "error",
      from: { path: "^src/shared/" },
      to: { path: "^src/(?:pages|widgets|app|dev)/" },
    },
    {
      name: "ui-does-not-own-feature-runtime",
      severity: "error",
      from: { path: "^src/shared/ui/" },
      to: { path: "^src/shared/(?:features|platform)/" },
    },
    {
      name: "pages-do-not-import-composition-roots",
      severity: "error",
      from: { path: "^src/pages/" },
      to: { path: "^src/(?:widgets|app|dev)/" },
    },
    {
      name: "widgets-do-not-import-pages-or-app",
      severity: "error",
      from: { path: "^src/widgets/" },
      to: { path: "^src/(?:pages|app|dev)/" },
    },
  ],
  options: {
    doNotFollow: { path: "node_modules" },
    tsConfig: { fileName: "tsconfig.json" },
    tsPreCompilationDeps: false,
  },
};
