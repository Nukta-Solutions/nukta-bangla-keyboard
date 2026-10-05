// riti's C API (riti-bridge/riti/include/riti.h) plus the bridge's own function.
// Link riti-bridge/lib/libnukta_riti.a: run scripts/build_riti.sh first.
#include "../../riti-bridge/riti/include/riti.h"

/// The riti keycode for a character typed on a US layout, or 0 if riti doesn't use it.
uint16_t nukta_riti_keycode(uint32_t ch);
