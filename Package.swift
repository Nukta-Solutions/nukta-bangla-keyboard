// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "BijoyBangla",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "BijoyBangla", targets: ["BijoyInputMethod"]),
    ],
    targets: [
        .target(name: "BijoyEngine"),
        .executableTarget(
            name: "BijoyInputMethod",
            dependencies: ["BijoyEngine"],
            linkerSettings: [.linkedFramework("InputMethodKit")]
        ),
        .testTarget(name: "BijoyEngineTests", dependencies: ["BijoyEngine"]),
    ]
)
