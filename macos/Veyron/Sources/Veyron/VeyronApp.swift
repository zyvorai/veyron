import SwiftUI

@main
struct VeyronApp: App {
    @StateObject private var model = AppModel()

    var body: some Scene {
        Window("Veyron", id: "main") {
            RootView()
                .environmentObject(model)
                .frame(minWidth: 1000, minHeight: 680)
                .onAppear { model.startPolling() }
        }
        .defaultSize(width: 1360, height: 860)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandGroup(after: .toolbar) {
                Button("Reload") { WebController.shared.reload() }.keyboardShortcut("r")
                Button("Back") { WebController.shared.back() }.keyboardShortcut("[")
                Button("Forward") { WebController.shared.forward() }.keyboardShortcut("]")
                Divider()
                Button("Zoom In") { WebController.shared.zoomIn() }.keyboardShortcut("=")
                Button("Zoom Out") { WebController.shared.zoomOut() }.keyboardShortcut("-")
                Button("Actual Size") { WebController.shared.zoomReset() }.keyboardShortcut("0")
            }
            CommandMenu("Server") {
                Button("Open in Browser") { model.openInBrowser() }
                    .keyboardShortcut("o", modifiers: [.command, .shift])
                    .disabled(model.server == nil)
                Button("Change Server…") { model.changeServer() }
                    .disabled(model.server == nil)
            }
        }

        Settings {
            SettingsView().environmentObject(model)
        }

        MenuBarExtra {
            MenuBarContent().environmentObject(model)
        } label: {
            Image(systemName: model.health.symbol)
        }
    }
}
