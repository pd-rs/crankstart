#!/usr/bin/env zsh
set -e
set -x

crankstart_crate_dir="$(cd "$(dirname "$0")/.." >/dev/null 2>&1 && pwd)"
source "$crankstart_crate_dir/scripts/vars.sh" || exit $?

common_params=("$crankstart_crate_dir/crankstart-sys/wrapper.h"
  "--use-core"
  "--ctypes-prefix"         "ctypes"
  "--with-derive-default"
  "--with-derive-eq"
  "--default-enum-style"    "rust"
  "--whitelist-type"        "PlaydateAPI"
  "--whitelist-type"        "PDSystemEvent"
  "--whitelist-type"        "LCDSolidColor"
  "--whitelist-type"        "LCDColor"
  "--whitelist-type"        "LCDPattern"
  "--whitelist-type"        "PDEventHandler"
  "--whitelist-var"         "LCD_COLUMNS"
  "--whitelist-var"         "LCD_ROWS"
  "--whitelist-var"         "LCD_ROWSIZE"
  "--whitelist-var"         "SEEK_SET"
  "--whitelist-var"         "SEEK_CUR"
  "--whitelist-var"         "SEEK_END"
  "--bitfield-enum"         "FileOptions"
  "--bitfield-enum"         "PDButtons"
)

bindgen $common_params \
  -- \
  -target x86_64-apple-darwin \
  -I"$PLAYDATE_C_API" \
  -DTARGET_EXTENSION > $crankstart_crate_dir/crankstart-sys/src/bindings_macos_x86.rs

bindgen $common_params \
  -- \
  -target aarch64-apple-darwin \
  -I"$PLAYDATE_C_API" \
  -DTARGET_EXTENSION > $crankstart_crate_dir/crankstart-sys/src/bindings_macos_aarch64.rs

bindgen $common_params \
  -- \
  -I"$PLAYDATE_C_API" \
  -I"/usr/local/playdate/gcc-arm-none-eabi-9-2019-q4-major/arm-none-eabi/include/" \
  -target thumbv7em-none-eabihf \
  -fshort-enums \
  -DTARGET_EXTENSION > $crankstart_crate_dir/crankstart-sys/src/bindings_playdate.rs
