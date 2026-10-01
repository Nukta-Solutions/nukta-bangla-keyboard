// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "NuktaBangla",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "NuktaBangla", targets: ["NuktaInputMethod"]),
    ],
    targets: [
        .target(name: "NuktaEngine"),
        .executableTarget(
            name: "NuktaInputMethod",
            dependencies: ["NuktaEngine"],
            linkerSettings: [.linkedFramework("InputMethodKit")]
        ),
        .testTarget(name: "NuktaEngineTests", dependencies: ["NuktaEngine"]),
    ]
)
