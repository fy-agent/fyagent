import Foundation

struct HelperSourceScanTests {
    static func helperMainAndServerHaveNoGenericRoutes() throws {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("Sources/FyAgentPrivilegedHelper")
        let files = try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "swift" }
        expect(Set(files.map(\.lastPathComponent)) == Set(["main.swift", "Server.swift"]))
        for file in files {
            let source = try String(contentsOf: file, encoding: .utf8)
            expect(!source.contains("Process"))
            expect(!source.contains("system("))
            expect(!source.contains("popen"))
            expect(!source.contains("ditto"))
            expect(!source.contains("NSAppleScript"))
            expect(!source.contains("SMAppService"))
            expect(!source.contains("sudo"))
        }
    }
}
