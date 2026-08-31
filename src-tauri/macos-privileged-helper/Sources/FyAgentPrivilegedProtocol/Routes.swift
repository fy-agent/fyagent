import Authorized
import Foundation
import SecureXPC

public struct KnownApplicationCommitRequest: Codable {
    public var fields: ClosedCommitFields
    public var sourceDirectory: FileHandleForXPC
    public var authorization: Authorization

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case operationId
        case action
        case product
        case targetSlot
        case expectedTargetRevision
        case expectedSourceRevision
        case reserved
        case sourceDirectory
        case authorization
    }

    public init(
        fields: ClosedCommitFields,
        sourceDirectory: FileHandleForXPC,
        authorization: Authorization
    ) {
        self.fields = fields
        self.sourceDirectory = sourceDirectory
        self.authorization = authorization
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        let operationId = try container.decode(UUID.self, forKey: .operationId)
        let action = try CommitAction(validating: try container.decode(UInt32.self, forKey: .action))
        let product = try KnownProduct(validating: try container.decode(UInt32.self, forKey: .product))
        let targetSlot = try container.decode(UInt32.self, forKey: .targetSlot)
        let expectedTargetRevision = try container.decode(Data.self, forKey: .expectedTargetRevision)
        let expectedSourceRevision = try container.decode(Data.self, forKey: .expectedSourceRevision)
        let reserved = try container.decode(UInt32.self, forKey: .reserved)
        fields = try ClosedCommitFields(
            protocolVersion: protocolVersion,
            operationId: operationId,
            action: action,
            product: product,
            targetSlot: targetSlot,
            expectedTargetRevision: expectedTargetRevision,
            expectedSourceRevision: expectedSourceRevision,
            reserved: reserved
        )
        sourceDirectory = try container.decode(FileHandleForXPC.self, forKey: .sourceDirectory)
        authorization = try container.decode(Authorization.self, forKey: .authorization)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(fields.protocolVersion, forKey: .protocolVersion)
        try container.encode(fields.operationId, forKey: .operationId)
        try container.encode(fields.action.rawValue, forKey: .action)
        try container.encode(fields.product.rawValue, forKey: .product)
        try container.encode(fields.targetSlot, forKey: .targetSlot)
        try container.encode(fields.expectedTargetRevision, forKey: .expectedTargetRevision)
        try container.encode(fields.expectedSourceRevision, forKey: .expectedSourceRevision)
        try container.encode(fields.reserved, forKey: .reserved)
        try container.encode(sourceDirectory, forKey: .sourceDirectory)
        try container.encode(authorization, forKey: .authorization)
    }
}

public struct AuthorizedRemoveHelperRequest: Codable {
    public var request: RemoveHelperRequest
    public var authorization: Authorization

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case operationId
        case reserved
        case authorization
    }

    public init(request: RemoveHelperRequest, authorization: Authorization) {
        self.request = request
        self.authorization = authorization
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        request = try RemoveHelperRequest(
            protocolVersion: try container.decode(UInt32.self, forKey: .protocolVersion),
            operationId: try container.decode(UUID.self, forKey: .operationId),
            reserved: try container.decode(UInt32.self, forKey: .reserved)
        )
        authorization = try container.decode(Authorization.self, forKey: .authorization)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(request.protocolVersion, forKey: .protocolVersion)
        try container.encode(request.operationId, forKey: .operationId)
        try container.encode(request.reserved, forKey: .reserved)
        try container.encode(authorization, forKey: .authorization)
    }
}

public enum PrivilegedRoutes {
    public static let status = XPCRoute
        .named("helper", "status")
        .withMessageType(HelperStatusRequest.self)
        .withReplyType(HelperStatusReply.self)

    public static let commit = XPCRoute
        .named("helper", "commitKnownApplication")
        .withMessageType(KnownApplicationCommitRequest.self)
        .withReplyType(KnownApplicationCommitResult.self)

    public static let remove = XPCRoute
        .named("helper", "removeHelper")
        .withMessageType(AuthorizedRemoveHelperRequest.self)
        .withReplyType(RemoveHelperResult.self)
}
