import Foundation

/// Turns whatever the user typed into the Veyron server they mean.
public enum ServerAddress {
    /// `80.79.5.173:30151`, `https://veyron.example.com/console`, `localhost:5151` ...
    /// Returns the origin (scheme, host, port) or nil when it cannot be a Veyron server.
    /// HTTPS is required except for loopback, so credentials never cross a network in clear.
    public static func normalize(_ raw: String) -> URL? {
        var text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, !text.contains(" ") else { return nil }
        if !text.contains("://") { text = "https://" + text }
        guard var parts = URLComponents(string: text),
              let scheme = parts.scheme?.lowercased(),
              let host = parts.host, !host.isEmpty
        else { return nil }
        guard scheme == "https" || (scheme == "http" && isLoopback(host)) else { return nil }
        parts.scheme = scheme
        parts.path = ""
        parts.query = nil
        parts.fragment = nil
        parts.user = nil
        parts.password = nil
        return parts.url
    }

    public static func isLoopback(_ host: String) -> Bool {
        let h = host.lowercased()
        return h == "localhost" || h == "127.0.0.1" || h == "::1" || h == "[::1]" || h.hasSuffix(".localhost")
    }

    public static func consoleURL(_ origin: URL) -> URL { origin.appendingPathComponent("console") }
    public static func healthURL(_ origin: URL) -> URL { origin.appendingPathComponent("api/v1/health") }

    /// `host:port` the way a person reads it.
    public static func display(_ origin: URL) -> String {
        guard let host = origin.host else { return origin.absoluteString }
        if let port = origin.port { return "\(host):\(port)" }
        return host
    }

    public static func sameOrigin(_ a: URL, _ b: URL) -> Bool {
        func port(_ u: URL) -> Int { u.port ?? (u.scheme?.lowercased() == "http" ? 80 : 443) }
        return a.scheme?.lowercased() == b.scheme?.lowercased()
            && a.host?.lowercased() == b.host?.lowercased()
            && port(a) == port(b)
    }
}
