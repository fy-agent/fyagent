import Darwin
import Foundation

enum DirectoryFDError: Error {
    case openFailed
    case notDirectory
    case unexpectedFileType
}

enum DirectoryFD {
    static func openDirectory(at url: URL) throws -> Int32 {
        try url.path.withCString { path in
            let fd = open(path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
            if fd < 0 {
                throw DirectoryFDError.openFailed
            }
            return fd
        }
    }

    static func close(_ fd: Int32) {
        if fd >= 0 {
            _ = Darwin.close(fd)
        }
    }

    static func fstat(_ fd: Int32) throws -> stat {
        var info = stat()
        if Darwin.fstat(fd, &info) != 0 {
            throw DirectoryFDError.openFailed
        }
        return info
    }

    static func requireDirectory(_ fd: Int32) throws {
        let info = try fstat(fd)
        guard (info.st_mode & S_IFMT) == S_IFDIR else {
            throw DirectoryFDError.notDirectory
        }
    }

    static func openAtDirectory(_ dirFD: Int32, _ name: String) throws -> Int32 {
        try name.withCString { cName in
            let fd = openat(dirFD, cName, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
            if fd < 0 {
                throw DirectoryFDError.openFailed
            }
            return fd
        }
    }

    static func openAtFile(_ dirFD: Int32, _ name: String) throws -> Int32 {
        try name.withCString { cName in
            let fd = openat(dirFD, cName, O_RDONLY | O_NOFOLLOW | O_CLOEXEC)
            if fd < 0 {
                throw DirectoryFDError.openFailed
            }
            let info = try fstat(fd)
            guard (info.st_mode & S_IFMT) == S_IFREG else {
                close(fd)
                throw DirectoryFDError.unexpectedFileType
            }
            return fd
        }
    }

    static func exists(_ dirFD: Int32, _ name: String) throws -> Bool {
        try name.withCString { cName in
            var info = stat()
            let result = fstatat(dirFD, cName, &info, AT_SYMLINK_NOFOLLOW)
            if result == 0 {
                return true
            }
            if errno == ENOENT {
                return false
            }
            throw DirectoryFDError.openFailed
        }
    }

    static func mkdirAt(_ dirFD: Int32, _ name: String, mode: mode_t = 0o755) throws {
        try name.withCString { cName in
            if mkdirat(dirFD, cName, mode) != 0 {
                throw DirectoryFDError.openFailed
            }
        }
    }

    static func renameAt(_ dirFD: Int32, from: String, to: String) throws {
        try from.withCString { fromName in
            try to.withCString { toName in
                if renameat(dirFD, fromName, dirFD, toName) != 0 {
                    throw DirectoryFDError.openFailed
                }
            }
        }
    }

    static func unlinkAt(_ dirFD: Int32, _ name: String, directory: Bool) throws {
        try name.withCString { cName in
            let flags = directory ? AT_REMOVEDIR : 0
            if unlinkat(dirFD, cName, flags) != 0 {
                throw DirectoryFDError.openFailed
            }
        }
    }

    static func fsync(_ fd: Int32) throws {
        if fcntl(fd, F_FULLFSYNC) == -1 {
            if Darwin.fsync(fd) != 0 {
                throw DirectoryFDError.openFailed
            }
        }
    }

    static func readFileAt(_ dirFD: Int32, _ name: String, limit: Int) throws -> Data {
        let fd = try openAtFile(dirFD, name)
        defer { close(fd) }
        var data = Data()
        var buffer = [UInt8](repeating: 0, count: 64 * 1024)
        while true {
            let n = buffer.withUnsafeMutableBytes { raw in
                Darwin.read(fd, raw.baseAddress, raw.count)
            }
            if n < 0 {
                throw DirectoryFDError.openFailed
            }
            if n == 0 {
                break
            }
            data.append(buffer, count: n)
            if data.count > limit {
                throw DirectoryFDError.openFailed
            }
        }
        return data
    }
}
