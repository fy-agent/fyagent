window.addEventListener("fyagent:open-auth-center", () => {
  void import("./legacy-auth-host.tsx").then((module) => {
    module.openLegacyAuthCenter();
  });
});
