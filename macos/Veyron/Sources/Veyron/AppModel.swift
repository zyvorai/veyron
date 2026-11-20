import AppKit
import SwiftUI
import VeyronCore

enum Health: Equatable {
    case unknown, online, offline

    var symbol: String {
        switch self {
        case .unknown: return "server.rack"
        case .online: return "checkmark.seal.fill"
        case .offline: return "exclamationmark.triangle.fill"
        }
    }
}

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var server: URL?
    @Published var loadError: String?
    @Published private(set) var health: Health = .unknown

    private let defaults = UserDefaults.standard
    private let probe = HealthProbe()
    private var timer: Timer?

    var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "dev"
    }

    var serverName: String { server.map(ServerAddress.display) ?? "No server" }

    init() {
        if let saved = defaults.string(forKey: "serverURL") {
            server = ServerAddress.normalize(saved)
        }
    }

    /// Returns a message when the address cannot be a Veyron server.
    @discardableResult
    func connect(_ raw: String) -> String? {
        guard let origin = ServerAddress.normalize(raw) else {
            return "Enter a host such as 80.79.5.173:30151 or veyron.example.com. HTTPS is required except for localhost."
        }
        server = origin
        loadError = nil
        defaults.set(origin.absoluteString, forKey: "serverURL")
        Task { await refreshHealth() }
        return nil
    }

    func changeServer() {
        server = nil
        loadError = nil
        health = .unknown
        defaults.removeObject(forKey: "serverURL")
    }

    func startPolling() {
        guard timer == nil else { return }
        Task { await refreshHealth() }
        timer = Timer.scheduledTimer(withTimeInterval: 30, repeats: true) { [weak self] _ in
            Task { @MainActor in await self?.refreshHealth() }
        }
    }

    func refreshHealth() async {
        guard let server else {
            health = .unknown
            return
        }
        health = await probe.check(server) ? .online : .offline
    }

    func openInBrowser() {
        if let server { NSWorkspace.shared.open(ServerAddress.consoleURL(server)) }
    }
}
