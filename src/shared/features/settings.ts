export type FeatureSettings = Record<string, unknown> & {
  skillSyncMethod?: "auto" | "symlink" | "copy";
  skillStorageLocation?: "fyagent" | "unified";
};
