// swift-tools-version:5.9
import PackageDescription
import Foundation

/// riti is a Rust static library built by scripts/build_riti.sh.
let ritiLibDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
    .appendingPathComponent("riti-bridge/lib").path

let package = Package(
    name: "NuktaBangla",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "NuktaBangla", targets: ["NuktaInputMethod"]),
    ],
    targets: [
        .target(name: "NuktaEngine"),
        .systemLibrary(name: "CRiti", path: "Sources/CRiti"),
        .target(
            name: "NuktaPhonetic",
            dependencies: ["CRiti"],
            linkerSettings: [.unsafeFlags(["-L", ritiLibDir])]
        ),
        .executableTarget(
            name: "NuktaInputMethod",
            dependencies: ["NuktaEngine", "NuktaPhonetic"],
            linkerSettings: [.linkedFramework("InputMethodKit")]
        ),
        .testTarget(name: "NuktaEngineTests", dependencies: ["NuktaEngine"]),
        .testTarget(name: "NuktaPhoneticTests", dependencies: ["NuktaPhonetic"]),
    ]
)
