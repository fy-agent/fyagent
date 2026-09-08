import Darwin
import Foundation

enum TestRuntime {
    static var failures = 0
    static var ran = 0

    static func expect(
        _ condition: Bool,
        _ message: @autoclosure () -> String = "",
        file: StaticString = #file,
        line: UInt = #line
    ) {
        if !condition {
            failures += 1
            fputs("FAIL \(file):\(line) \(message())\n", stderr)
        }
    }

    static func fail(_ message: String, file: StaticString = #file, line: UInt = #line) {
        failures += 1
        fputs("FAIL \(file):\(line) \(message)\n", stderr)
    }

    static func run(_ name: String, _ body: () throws -> Void) {
        ran += 1
        do {
            try body()
            fputs("ok   \(name)\n", stdout)
        } catch {
            failures += 1
            fputs("FAIL \(name): \(error)\n", stderr)
        }
    }
}

func expect(
    _ condition: Bool,
    _ message: @autoclosure () -> String = "",
    file: StaticString = #file,
    line: UInt = #line
) {
    TestRuntime.expect(condition, message(), file: file, line: line)
}

func expectThrows<E: Error & Equatable>(
    _ expected: E,
    file: StaticString = #file,
    line: UInt = #line,
    _ body: () throws -> Void
) {
    do {
        try body()
        TestRuntime.fail("expected to throw \(expected)", file: file, line: line)
    } catch let error as E {
        TestRuntime.expect(error == expected, "threw \(error) instead of \(expected)", file: file, line: line)
    } catch {
        TestRuntime.fail("threw \(error) instead of \(expected)", file: file, line: line)
    }
}

func expectThrowsAny(
    file: StaticString = #file,
    line: UInt = #line,
    _ body: () throws -> Void
) {
    do {
        try body()
        TestRuntime.fail("expected an error", file: file, line: line)
    } catch {
        // expected
    }
}
