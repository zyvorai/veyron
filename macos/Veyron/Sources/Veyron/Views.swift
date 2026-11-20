import SwiftUI
import VeyronCore

/// The Zyvor "Z" mark, drawn natively so it stays sharp at any size.
struct ZMark: Shape {
    func path(in rect: CGRect) -> Path {
        let u = min(rect.width, rect.height) / 18
        func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            CGPoint(x: rect.minX + x * u, y: rect.minY + y * u)
        }
        var path = Path()
        path.move(to: p(2, 2))
        path.addLine(to: p(16, 2))
        path.addLine(to: p(6.6, 16))
        path.addLine(to: p(16, 16))
        return path
    }
}

let zyvorOrange = Color(red: 1, green: 0.353, blue: 0.082)

struct RootView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            if let server = model.server {
                ConsoleView(server: server)
                if let error = model.loadError {
                    LoadErrorCard(message: error)
                }
            } else {
                ConnectView()
            }
        }
        .preferredColorScheme(.dark)
    }
}

struct ConnectView: View {
    @EnvironmentObject var model: AppModel
    @State private var address = ""
    @State private var problem: String?

    var body: some View {
        VStack(spacing: 22) {
            ZMark()
                .stroke(zyvorOrange, style: StrokeStyle(lineWidth: 2.6 * 5, lineCap: .round, lineJoin: .round))
                .frame(width: 90, height: 90)
                .shadow(color: zyvorOrange.opacity(0.45), radius: 18)
            VStack(spacing: 6) {
                Text("Connect to Veyron").font(.system(size: 26, weight: .semibold))
                Text("Enter the address of your Veyron server.")
                    .foregroundStyle(.secondary)
            }
            TextField("80.79.5.173:30151", text: $address)
                .textFieldStyle(.roundedBorder)
                .font(.system(size: 15, design: .monospaced))
                .frame(width: 320)
                .onSubmit(connect)
                .accessibilityLabel("Server address")
            if let problem {
                Text(problem).font(.callout).foregroundStyle(.red)
                    .frame(width: 360).multilineTextAlignment(.center)
            }
            Button("Connect", action: connect)
                .keyboardShortcut(.defaultAction)
                .buttonStyle(.borderedProminent)
                .tint(zyvorOrange)
                .controlSize(.large)
                .disabled(address.trimmingCharacters(in: .whitespaces).isEmpty)
            Text("A fresh Veyron install uses a self-signed certificate. The first time you connect, you will be asked to confirm its fingerprint.")
                .font(.footnote).foregroundStyle(.tertiary)
                .frame(width: 380).multilineTextAlignment(.center)
        }
        .padding(40)
    }

    private func connect() {
        problem = model.connect(address)
    }
}

struct LoadErrorCard: View {
    @EnvironmentObject var model: AppModel
    let message: String

    var body: some View {
        VStack(spacing: 14) {
            Image(systemName: "bolt.horizontal.circle").font(.system(size: 34)).foregroundStyle(zyvorOrange)
            Text("Can't reach \(model.serverName)").font(.headline)
            Text(message).font(.callout).foregroundStyle(.secondary)
                .multilineTextAlignment(.center).frame(maxWidth: 360)
            HStack {
                Button("Change Server") { model.changeServer() }
                Button("Try Again") {
                    model.loadError = nil
                    WebController.shared.reload()
                }
                .buttonStyle(.borderedProminent).tint(zyvorOrange)
            }
        }
        .padding(28)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
        .shadow(radius: 24)
    }
}

struct SettingsView: View {
    @EnvironmentObject var model: AppModel
    @State private var address = ""
    @State private var problem: String?
    @State private var pinnedFingerprint: String?

    var body: some View {
        Form {
            Section("Server") {
                TextField("Address", text: $address, prompt: Text("host:30151"))
                    .font(.system(.body, design: .monospaced))
                    .onSubmit(save)
                if let problem { Text(problem).font(.callout).foregroundStyle(.red) }
                HStack {
                    Button("Save", action: save).keyboardShortcut(.defaultAction)
                    Button("Disconnect") {
                        model.changeServer()
                        address = ""
                    }
                    .disabled(model.server == nil)
                }
            }
            Section("Trusted certificate") {
                if let fp = pinnedFingerprint, let host = model.server?.host {
                    Text(CertFingerprint.display(fp))
                        .font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                    Button("Forget certificate for \(host)") {
                        TrustHandler.shared.pins.forget(host)
                        pinnedFingerprint = nil
                    }
                } else {
                    Text("No pinned certificate. The server's own certificate is trusted by macOS, or you have not connected yet.")
                        .font(.callout).foregroundStyle(.secondary)
                }
            }
            Section {
                LabeledContent("Version", value: model.version)
            }
        }
        .formStyle(.grouped)
        .frame(width: 480, height: 400)
        .onAppear(perform: refresh)
        .onChange(of: model.server) { _ in refresh() }
    }

    private func refresh() {
        address = model.server.map(ServerAddress.display) ?? ""
        pinnedFingerprint = model.server?.host.flatMap { TrustHandler.shared.pins.pinned(for: $0) }
    }

    private func save() {
        problem = model.connect(address)
        if problem == nil { refresh() }
    }
}

struct MenuBarContent: View {
    @EnvironmentObject var model: AppModel
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Group {
            switch model.health {
            case .online: Text("\(model.serverName) is online")
            case .offline: Text("\(model.serverName) is unreachable")
            case .unknown: Text(model.server == nil ? "Not connected" : "Checking \(model.serverName)")
            }
        }
        Divider()
        Button("Open Veyron") {
            openWindow(id: "main")
            NSApp.activate(ignoringOtherApps: true)
        }
        Button("Open in Browser") { model.openInBrowser() }.disabled(model.server == nil)
        Button("Check Now") { Task { await model.refreshHealth() } }.disabled(model.server == nil)
        Divider()
        Button("Quit Veyron") { NSApp.terminate(nil) }.keyboardShortcut("q")
    }
}
