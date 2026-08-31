import Foundation

struct ForbiddenSurfaceTests {
    static func helperSourcesForbidProcessNetworkAndShell() throws {
        let roots = [
            packageRoot().appendingPathComponent("Sources/FyAgentPrivilegedHelper"),
            packageRoot().appendingPathComponent("Sources/FyAgentPrivilegedTransaction"),
        ]
        let forbidden = [
            "Process(",
            "Process.self",
            "NSTask",
            "system(",
            "popen(",
            "posix_spawn",
            "ditto",
            "curl",
            "URLSession",
            "/bin/sh",
            "AuthorizationExecuteWithPrivileges",
            "SMAppService",
        ]
        for root in roots {
            let files = try swiftFiles(in: root)
            expect(!files.isEmpty)
            for file in files {
                let source = try String(contentsOf: file, encoding: .utf8)
                for token in forbidden {
                    expect(!source.contains(token), "\(file.lastPathComponent) contains forbidden token \(token)")
                }
            }
        }
    }

    static func clientDoesNotPutAuthorizationBytesInCABI() throws {
        let header = try String(
            contentsOf: packageRoot().appendingPathComponent("include/fyagent_privileged_bridge.h"),
            encoding: .utf8
        )
        expect(!header.contains("Authorization"))
        expect(!header.contains("AuthorizationExternalForm"))
        expect(!header.contains("char *"))
        expect(!header.contains("source_path"))
        expect(!header.contains("target_path"))
        expect(!header.contains("argv"))
        expect(!header.contains("destination"))
        expect(!header.contains("http://"))
        expect(!header.contains("https://"))
    }
}

private func swiftFiles(in directory: URL) throws -> [URL] {
    try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
        .filter { $0.pathExtension == "swift" }
}

private func packageRoot() -> URL {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
}
