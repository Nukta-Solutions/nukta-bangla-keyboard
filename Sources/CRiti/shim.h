// riti's C API (riti-bridge/riti/include/riti.h) plus the bridge's own functions.
// Link riti-bridge/lib/libnukta_riti.a: run scripts/build_riti.sh first.
#include "../../riti-bridge/riti/include/riti.h"

/// The riti keycode for a character typed on a US layout, or 0 if riti doesn't use it.
uint16_t nukta_riti_keycode(uint32_t ch);

// The user's shortcuts (riti-bridge/src/shortcuts.rs). Strings are UTF-8; a returned string is
// the caller's, freed with nukta_string_free.
typedef struct NuktaShortcuts NuktaShortcuts;
NuktaShortcuts *nukta_shortcuts_new(const char *directory);
void nukta_shortcuts_free(NuktaShortcuts *shortcuts);
/// The text for what was typed if it is a whole code, else NULL.
char *nukta_shortcuts_text(NuktaShortcuts *shortcuts, const char *typed);
/// Every shortcut as a JSON object, code → text.
char *nukta_shortcuts_json(NuktaShortcuts *shortcuts);
/// 0 when saved; 1 bad code, 2 empty text, 3 text too long, 4 not saved.
int32_t nukta_shortcuts_set(NuktaShortcuts *shortcuts, const char *code, const char *text);
bool nukta_shortcuts_remove(NuktaShortcuts *shortcuts, const char *code);
/// A dictionary word the code also spells, else NULL.
char *nukta_dictionary_word(const char *code);
void nukta_string_free(char *text);
