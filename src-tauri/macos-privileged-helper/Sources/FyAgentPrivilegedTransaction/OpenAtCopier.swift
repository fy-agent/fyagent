import Darwin
import Foundation

enum CopierError: Error {
    case unsupportedFileType
    case symlinkEscape
    case tooDeep
    case tooManyEntries
    case tooLarge
    case copyFailed
}

enum OpenAtCopier {
    static let maxDepth = 64
    static let maxEntries = 100_000
    static let maxBytes: UInt64 = 2 * 1024 * 1024 * 1024

    static func copyBundle(fromSourceFD sourceFD: Int32, toParentFD parentFD: Int32, stageName: String) throws {
        try DirectoryFD.requireDirectory(sourceFD)
        if try DirectoryFD.exists(parentFD, stageName) {
            throw CopierError.copyFailed
        }
        try DirectoryFD.mkdirAt(parentFD, stageName)
        let destFD = try DirectoryFD.openAtDirectory(parentFD, stageName)
        defer { DirectoryFD.close(destFD) }
        var stats = CopyStats()
        try copyDirectory(from: sourceFD, to: destFD, relative: [], stats: &stats)
        try DirectoryFD.fsync(destFD)
        try DirectoryFD.fsync(parentFD)
    }

    private struct CopyStats {
        var entries = 0
        var bytes: UInt64 = 0
    }

    private static func copyDirectory(
        from sourceFD: Int32,
        to destFD: Int32,
        relative: [String],
        stats: inout CopyStats
    ) throws {
        if relative.count > maxDepth {
            throw CopierError.tooDeep
        }
        let listingFD = dup(sourceFD)
        guard listingFD >= 0 else { throw CopierError.copyFailed }
        guard let dir = fdopendir(listingFD) else {
            DirectoryFD.close(listingFD)
            throw CopierError.copyFailed
        }
        defer { closedir(dir) }

        while let entry = readdir(dir) {
            let name = direntName(entry.pointee)
            if name == "." || name == ".." {
                continue
            }
            if name.contains("/") || name.contains("\0") || name == "" {
                throw CopierError.copyFailed
            }
            stats.entries += 1
            if stats.entries > maxEntries {
                throw CopierError.tooManyEntries
            }
            try copyEntry(named: name, from: sourceFD, to: destFD, relative: relative, stats: &stats)
        }
    }

    private static func copyEntry(
        named name: String,
        from sourceFD: Int32,
        to destFD: Int32,
        relative: [String],
        stats: inout CopyStats
    ) throws {
        let opened = name.withCString { cName -> Int32 in
            openat(sourceFD, cName, O_RDONLY | O_NOFOLLOW | O_CLOEXEC)
        }
        if opened < 0 {
            if errno == ELOOP {
                try copySymlink(named: name, from: sourceFD, to: destFD, relative: relative)
                return
            }
            throw CopierError.copyFailed
        }
        defer { DirectoryFD.close(opened) }

        var info = stat()
        if fstat(opened, &info) != 0 {
            throw CopierError.copyFailed
        }
        let type = info.st_mode & S_IFMT
        if type == S_IFDIR {
            try DirectoryFD.mkdirAt(destFD, name, mode: info.st_mode & 0o777)
            let child = try DirectoryFD.openAtDirectory(destFD, name)
            defer { DirectoryFD.close(child) }
            try copyDirectory(from: opened, to: child, relative: relative + [name], stats: &stats)
            try DirectoryFD.fsync(child)
            return
        }
        if type == S_IFREG {
            try copyRegularFile(named: name, fromFD: opened, toDir: destFD, size: UInt64(info.st_size), stats: &stats)
            return
        }
        throw CopierError.unsupportedFileType
    }

    private static func copyRegularFile(
        named name: String,
        fromFD sourceFD: Int32,
        toDir destFD: Int32,
        size: UInt64,
        stats: inout CopyStats
    ) throws {
        stats.bytes += size
        if stats.bytes > maxBytes {
            throw CopierError.tooLarge
        }
        let destFile = name.withCString { cName in
            openat(destFD, cName, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0o644)
        }
        if destFile < 0 {
            throw CopierError.copyFailed
        }
        defer { DirectoryFD.close(destFile) }
        let status = fcopyfile(sourceFD, destFile, nil, copyfile_flags_t(COPYFILE_ALL))
        if status != 0 {
            throw CopierError.copyFailed
        }
        try DirectoryFD.fsync(destFile)
    }

    private static func copySymlink(
        named name: String,
        from sourceFD: Int32,
        to destFD: Int32,
        relative: [String]
    ) throws {
        var buffer = [CChar](repeating: 0, count: 4096)
        let length = name.withCString { cName in
            readlinkat(sourceFD, cName, &buffer, buffer.count - 1)
        }
        if length < 0 {
            throw CopierError.copyFailed
        }
        let target = String(cString: buffer)
        try validateSymlinkTarget(target, relative: relative)
        let result = name.withCString { cName in
            target.withCString { cTarget in
                symlinkat(cTarget, destFD, cName)
            }
        }
        if result != 0 {
            throw CopierError.copyFailed
        }
    }

    static func validateSymlinkTarget(_ target: String, relative: [String]) throws {
        if target.hasPrefix("/") {
            throw CopierError.symlinkEscape
        }
        var stack = relative
        for component in target.split(separator: "/", omittingEmptySubsequences: true).map(String.init) {
            if component == "." {
                continue
            }
            if component == ".." {
                if stack.isEmpty {
                    throw CopierError.symlinkEscape
                }
                stack.removeLast()
                continue
            }
            stack.append(component)
        }
    }

    private static func direntName(_ entry: dirent) -> String {
        withUnsafePointer(to: entry.d_name) { pointer in
            pointer.withMemoryRebound(to: CChar.self, capacity: Int(entry.d_namlen) + 1) { cString in
                String(cString: cString)
            }
        }
    }
}
