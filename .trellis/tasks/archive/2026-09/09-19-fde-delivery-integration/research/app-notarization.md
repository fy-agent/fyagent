# macOS App notarization sequence (Apple official check)

核对范围：只确认“先处理 `.app`、再制作最终 DMG”这条发布次序；资料来自 Apple Developer 文档，以及本机 Xcode `xcrun` 帮助。未执行真实上传或公证。

## 结论

1. **给 `.app` 提交公证时，先做 ZIP 归档；`ditto` 适用。** Apple 说明不能直接上传 `.app` bundle，需要先创建包含 app 的压缩归档；其示例使用：

   ```sh
   ditto -c -k --keepParent "MyApp.app" "MyApp.zip"
   xcrun notarytool submit "MyApp.zip" --keychain-profile "PROFILE" --wait
   ```

   `--keepParent` 保留 `.app` 的父目录层级，适合作为 app 的公证提交归档。Apple 同时支持直接提交 ZIP、PKG 或 DMG；这里选择 ZIP 是为了在公证通过后把 ticket staple 回原 `.app`。

2. **`notarytool submit --wait` 只保证等待处理结束，不单独等同于 Accepted。** 本机帮助将 `--wait` 定义为“等待直到处理完成”；脚本应读取最终输出（建议 `--output-format json` 或 plist），并把 `status` 严格判为 `Accepted` 才继续，`Rejected`、超时或命令失败都应停止。需要排查时用返回的 submission ID 调 `notarytool info` 获取状态和日志。

3. **App staple 后不需要重新签名。** Apple 的流程说明指出，提交前无需重新构建或重新签名；公证返回后把 ticket staple 到既有软件即可。因此顺序应是：签名 `.app` → ZIP 提交并确认 `Accepted` → `xcrun stapler staple MyApp.app` → 用 `codesign --verify` / `stapler validate` 做检查。staple 之后不要因为 ticket 再次签名；若之后修改了 app 内容，则那是新的构建，应重新签名并重新公证。

4. **最终要分发的 DMG 需要单独公证，并在通过后 staple DMG。** Apple 明确要求把已公证（并已 staple）的项目放入 installer/container 后，再按其他可执行分发物一样公证该 installer/container。故推荐：app 公证并 staple → 用该 app 制作最终 DMG → 对 DMG 签名 → `notarytool submit DMG --wait` 严格确认 `Accepted` → `xcrun stapler staple MyApp.dmg` → `xcrun stapler validate MyApp.dmg`。DMG 的 ticket 与 app 的 ticket 是两个分发层级，不能用 app 的 staple 代替 DMG 的公证。

## 本机命令帮助核对

- `xcrun notarytool submit --help`：`--wait/--no-wait` 的定义是“Wait until processing is complete”；`--timeout` 到时只停止等待，服务仍会继续处理。
- `xcrun stapler --help`：`stapler staple` 将 ticket 附加到支持的文件；支持签名 executable bundle 与 UDIF disk image；`stapler validate` 校验已 stapled ticket。

## 直接来源

- Apple Developer — [Customizing the notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)：不能直接上传 `.app`，需创建压缩归档；`ditto` ZIP 示例；`--wait` 等待服务完成；将已公证/已 staple 的项目放入 installer/container 后再公证 container。
- Apple Developer — [Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)：可提交 ZIP、installer package 或 disk image；提交前无需重新构建或重新签名；公证后 staple 返回的 ticket。
