import Cocoa
import InputMethodKit

let connectionName = Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String
    ?? "com.asifmahmud.inputmethod.NuktaBangla_Connection"

// Kept alive for the lifetime of the process.
let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier)

NSApplication.shared.run()
