import CryptoKit
import Foundation

/// A server certificate fingerprint: SHA-256 of the DER bytes, lower-case hex.
public enum CertFingerprint {
    public static func sha256Hex(of der: Data) -> String {
        SHA256.hash(data: der).map { String(format: "%02x", $0) }.joined()
    }

    /// `AB:CD:...` for showing to a person.
    public static func display(_ hex: String) -> String {
        stride(from: 0, to: hex.count, by: 2).map { i -> String in
            let s = hex.index(hex.startIndex, offsetBy: i)
            let e = hex.index(s, offsetBy: 2, limitedBy: hex.endIndex) ?? hex.endIndex
            return hex[s..<e].uppercased()
        }.joined(separator: ":")
    }
}

public enum TrustDecision: Equatable {
    case accept
    /// Never seen this self-signed certificate: ask the person to confirm the fingerprint.
    case askToTrust(fingerprint: String)
    /// A different certificate than the one trusted before: warn loudly.
    case certificateChanged(old: String, new: String)
}

public protocol PinStore: AnyObject {
    func pinned(for host: String) -> String?
    func pin(_ fingerprint: String, for host: String)
    func forget(_ host: String)
}

/// Trust-on-first-use for the self-signed certificate Veyron generates by default.
public enum CertificateTrust {
    public static func evaluate(presented: String, pinned: String?, systemTrusted: Bool) -> TrustDecision {
        // A certificate the system already trusts (a real CA) needs no pinning.
        if systemTrusted { return .accept }
        guard let pinned else { return .askToTrust(fingerprint: presented) }
        return pinned == presented ? .accept : .certificateChanged(old: pinned, new: presented)
    }
}

/// Pins live in UserDefaults: a fingerprint is public information, not a secret.
public final class DefaultsPinStore: PinStore {
    private let defaults: UserDefaults
    private let key = "pinnedCertificates"

    public init(defaults: UserDefaults = .standard) { self.defaults = defaults }

    private var all: [String: String] {
        get { defaults.dictionary(forKey: key) as? [String: String] ?? [:] }
        set { defaults.set(newValue, forKey: key) }
    }

    public func pinned(for host: String) -> String? { all[host.lowercased()] }
    public func pin(_ fingerprint: String, for host: String) { all[host.lowercased()] = fingerprint }
    public func forget(_ host: String) { all.removeValue(forKey: host.lowercased()) }
}
