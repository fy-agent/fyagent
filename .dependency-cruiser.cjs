/** Runtime architecture boundaries; TypeScript-only contracts do not create runtime edges. */
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
      name: "v2-does-not-import-leftover-renderer",
      severity: "error",
      from: { path: "^src/v2/" },
      to: { path: "^src/(?!v2/|shared/)" },
    },
    {
      name: "neutral-does-not-import-renderer-or-native-runtime",
      severity: "error",
      from: { path: "^src/shared/" },
      to: {
        path: "^src/(?!shared/)|(?:^|/)node_modules/(?:react(?:-dom)?|@tauri-apps)(?:/|$)",
      },
    },
    {
      name: "v2-shared-does-not-import-pages-or-widgets",
      severity: "error",
      from: { path: "^src/v2/shared/" },
      to: { path: "^src/v2/(?:pages|widgets)/" },
    },
    {
      name: "v2-ui-does-not-own-feature-runtime",
      severity: "error",
      from: { path: "^src/v2/shared/ui/" },
      to: { path: "^src/v2/shared/(?:features|platform)/" },
    },
  ],
  options: {
    doNotFollow: { path: "node_modules" },
    tsConfig: { fileName: "tsconfig.json" },
    tsPreCompilationDeps: false,
  },
};
