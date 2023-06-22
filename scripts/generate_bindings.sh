#!/bin/sh
set -e
set -x

crankstart_crate_dir="$(cd "$(dirname "$0")/.." >/dev/null 2>&1 && pwd)"
. "$crankstart_crate_dir/scripts/vars.sh"
# shellcheck disable=SC2181  # can't check exit code of . in same line with POSIX sh
if [ "$?" -ne 0 ]; then
   exit 2
fi

# POSIX sh "array" used to store common parameters to all bindgen calls
set -- "$crankstart_crate_dir/crankstart-sys/wrapper.h" \
  "--use-core" \
  "--ctypes-prefix"         "ctypes" \
  "--with-derive-default" \
  "--with-derive-eq" \
  "--default-enum-style"    "rust" \
  "--allowlist-type"        "PlaydateAPI" \
  "--allowlist-type"        "PDSystemEvent" \
  "--allowlist-type"        "LCDSolidColor" \
  "--allowlist-type"        "LCDColor" \
  "--allowlist-type"        "LCDPattern" \
  "--allowlist-type"        "PDEventHandler" \
  "--allowlist-var"         "LCD_COLUMNS" \
  "--allowlist-var"         "LCD_ROWS" \
  "--allowlist-var"         "LCD_ROWSIZE" \
  "--allowlist-var"         "SEEK_SET" \
  "--allowlist-var"         "SEEK_CUR" \
  "--allowlist-var"         "SEEK_END" \
  "--bitfield-enum"         "FileOptions" \
  "--bitfield-enum"         "PDButtons"

bindgen "$@" \
  -- \
  -target x86_64 \
  -I"$PLAYDATE_C_API" \
  -DTARGET_EXTENSION > "${crankstart_crate_dir}/crankstart-sys/src/bindings_x86.rs"

bindgen "$@" \
  -- \
  -target aarch64 \
  -I"$PLAYDATE_C_API" \
  -DTARGET_EXTENSION > "${crankstart_crate_dir}/crankstart-sys/src/bindings_aarch64.rs"

# /usr/include is here because under some versions of Linux (such as Mint 21.1)
# the arm-non-eabi-gcc -print-sysroot command outputs nothing
bindgen "$@" \
  -- \
  -I"$PLAYDATE_C_API" \
  -I"/usr/include" \
  -I"$(arm-none-eabi-gcc -print-sysroot)/include" \
  -target thumbv7em-none-eabihf \
  -fshort-enums \
  -DTARGET_EXTENSION > "${crankstart_crate_dir}/crankstart-sys/src/bindings_playdate.rs"
