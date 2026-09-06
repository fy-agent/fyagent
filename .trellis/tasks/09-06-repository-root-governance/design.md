# 根文件治理

config/收拢vite.config.ts、vitest.config.ts、playwright.config.ts、
playwright.performance.config.ts、dependency-cruiser.cjs；所有入口明确--config。
项目路径相对配置模块解析，Playwright明确webServer.cwd与testDir，Vitest各
环境保留原root/setup/include。PostCSS的唯一autoprefixer采用Vite内联现有插件，
移除根postcss.config.cjs；不新增依赖、空导出或代理文件。

保留eslint.config.mjs（CLI/编辑器自动发现）、tsconfig.json（TS项目根）以及
包/锁/环境/版本/许可证/README/贡献安全文档。根.DS_Store确认仅Finder元数据
后删除；绝不使用git clean批量清理用户文件。新增根目录合同拦截无归属脚本。

同步任务运行器、工具检查、CI分类、测试、现行SPEC和有效归档上下文；历史
说明与Git对象不重写。移位使用Git可识别内容连续性，不复制第二套配置。
