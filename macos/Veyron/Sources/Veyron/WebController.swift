import WebKit

/// The one console web view, so menu commands can reach it.
@MainActor
final class WebController {
    static let shared = WebController()
    weak var webView: WKWebView?
    private let zoomKey = "pageZoom"

    func reload() { webView?.reloadFromOrigin() }
    func back() { webView?.goBack() }
    func forward() { webView?.goForward() }

    var zoom: CGFloat {
        get {
            let z = UserDefaults.standard.double(forKey: zoomKey)
            return z == 0 ? 1 : CGFloat(z)
        }
        set {
            let z = min(max(newValue, 0.5), 2.5)
            UserDefaults.standard.set(Double(z), forKey: zoomKey)
            webView?.pageZoom = z
        }
    }

    func zoomIn() { zoom += 0.1 }
    func zoomOut() { zoom -= 0.1 }
    func zoomReset() { zoom = 1 }
}
