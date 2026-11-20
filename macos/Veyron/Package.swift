// swift-tools-version: 6.0
import PackageDescription

// Veyron for macOS: a native window around the Veyron console.
// `VeyronCore` is pure logic (server addresses, navigation policy, certificate pinning)
// so it can be unit-tested without a GUI; `Veyron` is the SwiftUI/WebKit app.
let package = Package(
    name: "Veyron",
    platforms: [.macOS(.v13)],
    products: [.executable(name: "Veyron", targets: ["Veyron"])],
    targets: [
        .target(name: "VeyronCore"),
        .executableTarget(name: "Veyron", dependencies: ["VeyronCore"]),
        .testTarget(name: "VeyronCoreTests", dependencies: ["VeyronCore"]),
    ],
    swiftLanguageModes: [.v5]
)
