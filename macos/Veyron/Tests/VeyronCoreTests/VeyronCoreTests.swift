import XCTest
@testable import VeyronCore

final class ServerAddressTests: XCTestCase {
    func testNormalizesWhatPeopleType() {
        XCTAssertEqual(ServerAddress.normalize("80.79.5.173:30151")?.absoluteString, "https://80.79.5.173:30151")
        XCTAssertEqual(ServerAddress.normalize("  veyron.example.com  ")?.absoluteString, "https://veyron.example.com")
        XCTAssertEqual(ServerAddress.normalize("https://veyron.example.com/console#/vms")?.absoluteString, "https://veyron.example.com")
        XCTAssertEqual(ServerAddress.normalize("HTTPS://Host.Example:5151/api?x=1")?.absoluteString, "https://Host.Example:5151")
    }

    func testRefusesClearTextAndGarbage() {
        XCTAssertNil(ServerAddress.normalize("http://veyron.example.com"), "http only for loopback")
        XCTAssertEqual(ServerAddress.normalize("http://localhost:5151")?.absoluteString, "http://localhost:5151")
        XCTAssertEqual(ServerAddress.normalize("http://127.0.0.1:5151")?.absoluteString, "http://127.0.0.1:5151")
        for bad in ["", "   ", "not a url", "ftp://host", "javascript:alert(1)", "https://"] {
            XCTAssertNil(ServerAddress.normalize(bad), bad)
        }
    }

    func testDropsCredentialsInTheURL() {
        XCTAssertEqual(ServerAddress.normalize("https://admin:secret@host.example")?.absoluteString, "https://host.example")
    }

    func testURLsAndDisplay() {
        let o = ServerAddress.normalize("host.example:30151")!
        XCTAssertEqual(ServerAddress.consoleURL(o).absoluteString, "https://host.example:30151/console")
        XCTAssertEqual(ServerAddress.healthURL(o).absoluteString, "https://host.example:30151/api/v1/health")
        XCTAssertEqual(ServerAddress.display(o), "host.example:30151")
        XCTAssertEqual(ServerAddress.display(ServerAddress.normalize("host.example")!), "host.example")
    }

    func testSameOriginComparesSchemeHostAndPort() {
        let a = URL(string: "https://host.example:30151/console")!
        XCTAssertTrue(ServerAddress.sameOrigin(a, URL(string: "https://HOST.example:30151/api/v1/vms")!))
        XCTAssertFalse(ServerAddress.sameOrigin(a, URL(string: "https://host.example:5151/")!))
        XCTAssertFalse(ServerAddress.sameOrigin(a, URL(string: "http://host.example:30151/")!))
        XCTAssertFalse(ServerAddress.sameOrigin(a, URL(string: "https://evil.example:30151/")!))
        XCTAssertTrue(ServerAddress.sameOrigin(URL(string: "https://h.example")!, URL(string: "https://h.example:443")!))
    }
}

final class NavigationPolicyTests: XCTestCase {
    let server = URL(string: "https://host.example:30151")!

    func testOwnServerLoadsInTheApp() {
        let u = URL(string: "https://host.example:30151/console#/vms")
        XCTAssertEqual(NavigationPolicy.decide(url: u, server: server, isMainFrame: true), .allow)
    }

    func testOtherSitesOpenInTheBrowser() {
        let u = URL(string: "https://example.com/docs")
        XCTAssertEqual(NavigationPolicy.decide(url: u, server: server, isMainFrame: true), .openExternally)
        XCTAssertEqual(NavigationPolicy.decide(url: u, server: server, isMainFrame: false), .allow, "subframes are not hijacked")
        XCTAssertEqual(NavigationPolicy.decide(url: URL(string: "mailto:a@b.c"), server: server, isMainFrame: true), .openExternally)
    }

    func testOddSchemesAreRefused() {
        for s in ["file:///etc/passwd", "javascript:alert(1)", "ssh://host", "x-apple.systempreferences:"] {
            XCTAssertEqual(NavigationPolicy.decide(url: URL(string: s), server: server, isMainFrame: true), .cancel, s)
        }
        XCTAssertEqual(NavigationPolicy.decide(url: nil, server: server, isMainFrame: true), .cancel)
        XCTAssertEqual(NavigationPolicy.decide(url: URL(string: "blob:https://host.example:30151/abc"), server: server, isMainFrame: true), .allow)
    }
}

final class CertificateTrustTests: XCTestCase {
    func testFingerprintsAreStableHexAndReadable() {
        let fp = CertFingerprint.sha256Hex(of: Data("veyron".utf8))
        XCTAssertEqual(fp.count, 64)
        XCTAssertEqual(fp, fp.lowercased())
        XCTAssertEqual(fp, CertFingerprint.sha256Hex(of: Data("veyron".utf8)))
        XCTAssertNotEqual(fp, CertFingerprint.sha256Hex(of: Data("veyroN".utf8)))
        XCTAssertEqual(CertFingerprint.display("abcdef01"), "AB:CD:EF:01")
    }

    func testTrustOnFirstUseThenPinned() {
        XCTAssertEqual(CertificateTrust.evaluate(presented: "aa", pinned: nil, systemTrusted: false), .askToTrust(fingerprint: "aa"))
        XCTAssertEqual(CertificateTrust.evaluate(presented: "aa", pinned: "aa", systemTrusted: false), .accept)
    }

    func testAChangedCertificateIsNeverSilentlyAccepted() {
        XCTAssertEqual(
            CertificateTrust.evaluate(presented: "bb", pinned: "aa", systemTrusted: false),
            .certificateChanged(old: "aa", new: "bb"))
    }

    func testARealCertificateNeedsNoPin() {
        XCTAssertEqual(CertificateTrust.evaluate(presented: "aa", pinned: nil, systemTrusted: true), .accept)
        XCTAssertEqual(CertificateTrust.evaluate(presented: "bb", pinned: "aa", systemTrusted: true), .accept)
    }

    func testPinStoreRoundTripsAndForgets() {
        let suite = UserDefaults(suiteName: "veyron.tests.\(UUID().uuidString)")!
        let store = DefaultsPinStore(defaults: suite)
        XCTAssertNil(store.pinned(for: "Host.Example"))
        store.pin("aa", for: "Host.Example")
        XCTAssertEqual(store.pinned(for: "host.example"), "aa", "hosts compare case-insensitively")
        store.forget("HOST.example")
        XCTAssertNil(store.pinned(for: "host.example"))
    }
}
