import Authorized
import Foundation
import FyAgentPrivilegedProtocol
import Security

enum PrivilegedRights {
    static let commit = AuthorizationRight(name: PrivilegedIdentifiers.commitRightName)
    static let remove = AuthorizationRight(name: PrivilegedIdentifiers.removeRightName)

    /// Cached admin credentials must outlive the XPC hop into a LaunchDaemon.
    /// Canned `authenticateAsAdmin` uses timeout 0: the app can succeed off a
    /// shared Bless credential in milliseconds, then the daemon recheck fails
    /// with `errAuthorizationInteractionNotAllowed`.
    static let cachedCredentialTimeoutSeconds = 300

    /// App-side flags for a right that a LaunchDaemon will later extend.
    static let clientRequestOptions: Set<AuthorizationOption> = [
        .interactionAllowed,
        .extendRights,
        .preAuthorize,
    ]
    static let helperRecheckOptions: Set<AuthorizationOption> = [.extendRights]

    static func requestCommit() throws -> Authorization {
        try requestOnAppMain([commit])
    }

    static func requestRemove() throws -> Authorization {
        try requestOnAppMain([remove])
    }

    static func recheck(_ authorization: Authorization, right: AuthorizationRight) throws {
        _ = try authorization.requestRights(
            [right],
            environment: [],
            options: helperRecheckOptions
        )
    }

    /// Security Agent UI is scheduled from the application main run loop.
    /// The C ABI is invoked from a Tokio blocking pool thread; requesting rights
    /// there delays the password prompt and can starve the dialog.
    static func runOnAppMain<T>(_ work: @escaping () throws -> T) throws -> T {
        if Thread.isMainThread {
            return try work()
        }
        let box = MainResultBox<T>()
        let lock = DispatchSemaphore(value: 0)
        DispatchQueue.main.async {
            box.store(Result { try work() })
            lock.signal()
        }
        lock.wait()
        return try box.take()
    }

    private static func requestOnAppMain(_ rights: Set<AuthorizationRight>) throws -> Authorization {
        try runOnAppMain {
            let authorization = try Authorization()
            try ensureDefined(using: authorization)
            let granted = try authorization.requestRights(
                rights,
                environment: [],
                options: clientRequestOptions
            )
            if granted.contains(where: \.cannotPreAuthorize) {
                throw AuthorizationError.interactionNotAllowed
            }
            return authorization
        }
    }

    private static func ensureDefined(using authorization: Authorization) throws {
        try defineUserAdminRight(
            commit,
            authorization: authorization,
            descriptionKey: "FyAgent needs administrator authorization to install or update a known application in /Applications.",
            comment: "Per-operation right for known-application system commit."
        )
        try defineUserAdminRight(
            remove,
            authorization: authorization,
            descriptionKey: "FyAgent needs administrator authorization to remove its privileged helper.",
            comment: "Per-operation right for privileged helper removal."
        )
    }

    private static func defineUserAdminRight(
        _ right: AuthorizationRight,
        authorization: Authorization,
        descriptionKey: String,
        comment: String
    ) throws {
        if let definition = try right.retrieveDefinition(),
           let timeout = definition.timeout,
           timeout >= 1,
           definition.shared == true {
            return
        }
        try right.name.withCString { namePointer in
            let definition: [String: Any] = [
                "class": "user",
                "group": "admin",
                "authenticate-user": true,
                "shared": true,
                "timeout": cachedCredentialTimeoutSeconds,
                kAuthorizationComment as String: comment,
            ]
            let status = AuthorizationRightSet(
                authorization.authorizationRef,
                namePointer,
                definition as CFDictionary,
                descriptionKey as CFString,
                nil,
                nil
            )
            if status == errAuthorizationCanceled {
                throw AuthorizationError.canceled
            }
            if status != errAuthorizationSuccess {
                throw AuthorizationError.other(status)
            }
        }
    }
}

private final class MainResultBox<T> {
    private let lock = NSLock()
    private var result: Result<T, Error>?

    func store(_ result: Result<T, Error>) {
        lock.lock()
        self.result = result
        lock.unlock()
    }

    func take() throws -> T {
        lock.lock()
        defer { lock.unlock() }
        switch result {
        case .success(let value):
            return value
        case .failure(let error):
            throw error
        case .none:
            throw AuthorizationError.internalError
        }
    }
}
