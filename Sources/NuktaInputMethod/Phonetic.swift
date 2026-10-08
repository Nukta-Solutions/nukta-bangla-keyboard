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
        panelUpdate += 1
        guard composer.isComposing, !composer.candidates.isEmpty else {
            panel.hide()
            return
        }
        let (cursor, exact) = CursorRect.of(client)
        showPanel(at: cursor)
        guard !exact else { return }

        // The app doesn't know where the word is yet (Chrome, for the first letters of a word):
        // ask again shortly, and move the list there once it does.
        let update = panelUpdate
        for delay in [0.03, 0.08, 0.15, 0.3, 0.5, 0.8, 1.2] {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self, weak client] in
                guard let self, let client, update == self.panelUpdate, self.composer.isComposing,
                      let rect = CursorRect.exact(client) else { return }
                self.panelUpdate += 1
                self.showPanel(at: rect)
            }
        }
    }

    /// Counts panel updates, so a late answer for an earlier key can't move the list.
    private var panelUpdate = 0

    private func showPanel(at cursor: NSRect) {
        panel.show(candidates: composer.candidates, auxiliary: composer.auxiliary,
                   selected: composer.selectedIndex, cursor: cursor,
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
