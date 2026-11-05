import AppKit
import Security
import VeyronCore

/// Decides whether to trust the server's TLS certificate, for the web view and the health probe.
/// Veyron serves a self-signed certificate by default, so a certificate the system does not
/// trust is accepted only after the person confirms its fingerprint (trust on first use),
/// and a certificate that later changes is never accepted silently.
final class TrustHandler {
    static let shared = TrustHandler(pins: DefaultsPinStore())
    let pins: PinStore

    init(pins: PinStore) { self.pins = pins }

    func handle(
        _ challenge: URLAuthenticationChallenge,
        interactive: Bool,
        completion: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        let space = challenge.protectionSpace
        guard space.authenticationMethod == NSURLAuthenticationMethodServerTrust, let trust = space.serverTrust else {
            completion(.performDefaultHandling, nil)
            return
        }
        guard let leaf = (SecTrustCopyCertificateChain(trust) as? [SecCertificate])?.first else {
            completion(.cancelAuthenticationChallenge, nil)
            return
        }
        let host = space.host
        let fingerprint = CertFingerprint.sha256Hex(of: SecCertificateCopyData(leaf) as Data)
        let systemTrusted = SecTrustEvaluateWithError(trust, nil)
        let credential = URLCredential(trust: trust)

        switch CertificateTrust.evaluate(presented: fingerprint, pinned: pins.pinned(for: host), systemTrusted: systemTrusted) {
        case .accept:
            completion(.useCredential, credential)
        case .askToTrust, .certificateChanged:
            guard interactive else {
                completion(.cancelAuthenticationChallenge, nil)
                return
            }
            DispatchQueue.main.async {
                // Several resources can challenge at once; the first answer settles the rest.
                if self.pins.pinned(for: host) == fingerprint {
                    completion(.useCredential, credential)
                    return
                }
                let previous = self.pins.pinned(for: host)
                if Self.confirm(host: host, fingerprint: fingerprint, previous: previous) {
                    self.pins.pin(fingerprint, for: host)
                    completion(.useCredential, credential)
                } else {
                    completion(.cancelAuthenticationChallenge, nil)
                }
            }
        }
    }

    private static func confirm(host: String, fingerprint: String, previous: String?) -> Bool {
        let alert = NSAlert()
        let shown = CertFingerprint.display(fingerprint)
        if let previous {
            alert.alertStyle = .critical
            alert.messageText = "The certificate for \(host) has changed"
            alert.informativeText = """
            Veyron remembered a different certificate for this server. That is expected if you \
            regenerated it, but it can also mean someone is intercepting the connection.

            New SHA-256 fingerprint:
            \(shown)

            Previously trusted:
            \(CertFingerprint.display(previous))
            """
            alert.addButton(withTitle: "Cancel")
            alert.addButton(withTitle: "Trust New Certificate")
        } else {
            alert.alertStyle = .warning
            alert.messageText = "Trust the certificate for \(host)?"
            alert.informativeText = """
            This server's certificate is not signed by an authority macOS trusts. That is normal \
            for a fresh Veyron install, which creates its own. Continue only if this fingerprint \
            matches your server.

            SHA-256 fingerprint:
            \(shown)
            """
            alert.addButton(withTitle: "Trust Certificate")
            alert.addButton(withTitle: "Cancel")
        }
        let response = alert.runModal()
        return previous == nil ? response == .alertFirstButtonReturn : response == .alertSecondButtonReturn
    }
}

/// Probes `/api/v1/health` with the same trust rules, but never prompts.
final class HealthProbe: NSObject, URLSessionDelegate {
    private lazy var session = URLSession(configuration: .ephemeral, delegate: self, delegateQueue: nil)

    func check(_ origin: URL) async -> Bool {
        var request = URLRequest(url: ServerAddress.healthURL(origin))
        request.timeoutInterval = 6
        guard let (_, response) = try? await session.data(for: request) else { return false }
        return (response as? HTTPURLResponse)?.statusCode == 200
    }

    func urlSession(
        _ session: URLSession,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        TrustHandler.shared.handle(challenge, interactive: false, completion: completionHandler)
    }
}
