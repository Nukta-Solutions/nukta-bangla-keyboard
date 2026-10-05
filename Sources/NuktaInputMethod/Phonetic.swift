import Cocoa
import InputMethodKit
import NuktaPhonetic

/// Phonetic typing for the whole process: one composer (riti is large, and keeps one file of
/// learned picks) and one suggestion list. Every app gets its own input controller, but only one
/// types at a time: the `owner`.
final class Phonetic {
    static var shared: Phonetic {
        if let loaded { return loaded }
        let phonetic = Phonetic()
        loaded = phonetic
        return phonetic
    }
    /// Nil until phonetic typing is first used, so Bijoy never loads riti.
    private(set) static var loaded: Phonetic?

    let composer: PhoneticComposer
    let panel = CandidatePanel()
    private(set) weak var owner: NuktaInputController?

    private init() {
        composer = PhoneticComposer(options: Settings.phoneticOptions, directory: Settings.dataDirectory)
        composer.horizontalNavigation = Settings.popupDirection == .horizontal
        panel.onSelect = { [weak self] index in self?.owner?.pickCandidate(at: index) }
        NotificationCenter.default.addObserver(self, selector: #selector(settingsChanged),
                                               name: .nuktaSettingsChanged, object: nil)
    }

    func isOwner(_ controller: NuktaInputController) -> Bool {
        owner === controller
    }

    /// Makes `controller` the one typing. A word left in progress by the previous owner is
    /// committed to its own app first.
    func claim(_ controller: NuktaInputController) {
        guard owner !== controller else { return }
        owner?.phoneticTakenOver()
        composer.discard()
        panel.hide()
        owner = controller
    }

    /// Shows the list for the word in progress under (or over) it, or hides it.
    func updatePanel(for client: Client) {
        guard composer.isComposing, !composer.candidates.isEmpty else {
            panel.hide()
            return
        }
        panel.show(candidates: composer.candidates, auxiliary: composer.auxiliary,
                   selected: composer.selectedIndex, cursor: CursorRect.of(client),
                   position: Settings.popupPosition, direction: Settings.popupDirection)
    }

    @objc private func settingsChanged() {
        composer.horizontalNavigation = Settings.popupDirection == .horizontal
        let options = Settings.phoneticOptions
        guard options != composer.options else { return }
        let wasComposing = composer.isComposing
        composer.update(options: options)
        if wasComposing, !composer.isComposing {
            // riti was rebuilt: the word in progress is gone.
            owner?.clearComposition()
            panel.hide()
        }
    }
}
