import Foundation

public enum NavigationDecision: Equatable {
    case allow
    /// Hand the link to the user's default browser instead of loading it in the app.
    case openExternally
    case cancel
}

/// The app only ever shows the Veyron console. Everything else opens in the browser,
/// so a link in an alert description can never turn the window into a general web view.
public enum NavigationPolicy {
    public static func decide(url: URL?, server: URL, isMainFrame: Bool) -> NavigationDecision {
        guard let url, let scheme = url.scheme?.lowercased() else { return .cancel }
        switch scheme {
        case "about", "blob", "data":
            return .allow
        case "http", "https":
            if ServerAddress.sameOrigin(url, server) { return .allow }
            return isMainFrame ? .openExternally : .allow
        case "mailto", "tel":
            return .openExternally
        default:
            return .cancel
        }
    }
}
