import AppKit
import SwiftUI
import UniformTypeIdentifiers
import VeyronCore
import WebKit

/// The Veyron console in a WKWebView, with the native touches a browser tab lacks:
/// file pickers for image uploads, downloads to disk, JS dialogs, and certificate pinning.
struct ConsoleView: NSViewRepresentable {
    let server: URL
    @EnvironmentObject var model: AppModel

    func makeCoordinator() -> Coordinator { Coordinator(model: model, server: server) }

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .default()   // keeps the console login between launches
        config.preferences.isElementFullscreenEnabled = true   // VNC / serial consoles
        config.applicationNameForUserAgent = "VeyronMac/\(model.version)"
        let web = WKWebView(frame: .zero, configuration: config)
        web.navigationDelegate = context.coordinator
        web.uiDelegate = context.coordinator
        web.allowsBackForwardNavigationGestures = true
        web.underPageBackgroundColor = .black
        web.pageZoom = WebController.shared.zoom
        #if DEBUG
        if #available(macOS 13.3, *) { web.isInspectable = true }
        #endif
        WebController.shared.webView = web
        context.coordinator.load(in: web)
        return web
    }

    func updateNSView(_ web: WKWebView, context: Context) {
        context.coordinator.server = server
        context.coordinator.loadIfServerChanged(in: web)
    }

    final class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate, WKDownloadDelegate {
        let model: AppModel
        var server: URL
        private var loadedServer: URL?
        private var downloadTargets: [ObjectIdentifier: URL] = [:]

        init(model: AppModel, server: URL) {
            self.model = model
            self.server = server
        }

        func load(in web: WKWebView) {
            loadedServer = server
            web.load(URLRequest(url: ServerAddress.consoleURL(server)))
        }

        func loadIfServerChanged(in web: WKWebView) {
            if loadedServer != server { load(in: web) }
        }

        // MARK: navigation

        func webView(
            _ webView: WKWebView,
            decidePolicyFor action: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            if action.shouldPerformDownload {
                decisionHandler(.download)
                return
            }
            switch NavigationPolicy.decide(url: action.request.url, server: server, isMainFrame: action.targetFrame?.isMainFrame ?? true) {
            case .allow:
                decisionHandler(.allow)
            case .openExternally:
                if let url = action.request.url { NSWorkspace.shared.open(url) }
                decisionHandler(.cancel)
            case .cancel:
                decisionHandler(.cancel)
            }
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor response: WKNavigationResponse,
            decisionHandler: @escaping (WKNavigationResponsePolicy) -> Void
        ) {
            decisionHandler(response.canShowMIMEType ? .allow : .download)
        }

        func webView(_ webView: WKWebView, navigationAction: WKNavigationAction, didBecome download: WKDownload) {
            download.delegate = self
        }

        func webView(_ webView: WKWebView, navigationResponse: WKNavigationResponse, didBecome download: WKDownload) {
            download.delegate = self
        }

        func webView(
            _ webView: WKWebView,
            didReceive challenge: URLAuthenticationChallenge,
            completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
        ) {
            TrustHandler.shared.handle(challenge, interactive: true, completion: completionHandler)
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            model.loadError = nil
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
            report(error)
        }

        func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
            report(error)
        }

        private func report(_ error: Error) {
            let e = error as NSError
            if e.domain == NSURLErrorDomain && e.code == NSURLErrorCancelled { return }
            if e.domain == "WebKitErrorDomain" && e.code == 102 { return }   // frame load interrupted (download)
            model.loadError = e.localizedDescription
        }

        func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
            webView.reload()
        }

        // MARK: new windows (target=_blank)

        func webView(
            _ webView: WKWebView,
            createWebViewWith configuration: WKWebViewConfiguration,
            for action: WKNavigationAction,
            windowFeatures: WKWindowFeatures
        ) -> WKWebView? {
            guard let url = action.request.url else { return nil }
            if ServerAddress.sameOrigin(url, server) {
                webView.load(action.request)
            } else if NavigationPolicy.decide(url: url, server: server, isMainFrame: true) == .openExternally {
                NSWorkspace.shared.open(url)
            }
            return nil
        }

        // MARK: JavaScript dialogs (the console uses prompt() for a few actions)

        func webView(
            _ webView: WKWebView,
            runJavaScriptAlertPanelWithMessage message: String,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping () -> Void
        ) {
            let alert = NSAlert()
            alert.messageText = message
            alert.addButton(withTitle: "OK")
            alert.runModal()
            completionHandler()
        }

        func webView(
            _ webView: WKWebView,
            runJavaScriptConfirmPanelWithMessage message: String,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping (Bool) -> Void
        ) {
            let alert = NSAlert()
            alert.messageText = message
            alert.addButton(withTitle: "OK")
            alert.addButton(withTitle: "Cancel")
            completionHandler(alert.runModal() == .alertFirstButtonReturn)
        }

        func webView(
            _ webView: WKWebView,
            runJavaScriptTextInputPanelWithPrompt prompt: String,
            defaultText: String?,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping (String?) -> Void
        ) {
            let alert = NSAlert()
            alert.messageText = prompt
            let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 300, height: 24))
            field.stringValue = defaultText ?? ""
            alert.accessoryView = field
            alert.addButton(withTitle: "OK")
            alert.addButton(withTitle: "Cancel")
            alert.window.initialFirstResponder = field
            completionHandler(alert.runModal() == .alertFirstButtonReturn ? field.stringValue : nil)
        }

        // MARK: file picker (image and ISO uploads)

        func webView(
            _ webView: WKWebView,
            runOpenPanelWith parameters: WKOpenPanelParameters,
            initiatedByFrame frame: WKFrameInfo,
            completionHandler: @escaping ([URL]?) -> Void
        ) {
            let panel = NSOpenPanel()
            panel.allowsMultipleSelection = parameters.allowsMultipleSelection
            panel.canChooseDirectories = parameters.allowsDirectories
            panel.begin { completionHandler($0 == .OK ? panel.urls : nil) }
        }

        // MARK: downloads

        func download(
            _ download: WKDownload,
            decideDestinationUsing response: URLResponse,
            suggestedFilename: String,
            completionHandler: @escaping (URL?) -> Void
        ) {
            let panel = NSSavePanel()
            panel.nameFieldStringValue = suggestedFilename
            panel.directoryURL = FileManager.default.urls(for: .downloadsDirectory, in: .userDomainMask).first
            panel.begin { [weak self] result in
                guard result == .OK, let url = panel.url else {
                    completionHandler(nil)
                    return
                }
                self?.downloadTargets[ObjectIdentifier(download)] = url
                completionHandler(url)
            }
        }

        func downloadDidFinish(_ download: WKDownload) {
            if let url = downloadTargets.removeValue(forKey: ObjectIdentifier(download)) {
                NSWorkspace.shared.activateFileViewerSelecting([url])
            }
        }

        func download(_ download: WKDownload, didFailWithError error: Error, resumeData: Data?) {
            downloadTargets.removeValue(forKey: ObjectIdentifier(download))
            let alert = NSAlert()
            alert.alertStyle = .warning
            alert.messageText = "Download failed"
            alert.informativeText = error.localizedDescription
            alert.runModal()
        }
    }
}
