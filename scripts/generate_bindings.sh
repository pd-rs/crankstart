#!/usr/bin/env zsh
set -e
set -x

crankstart_crate_dir="$(cd "$(dirname "$0")/.." >/dev/null 2>&1 && pwd)"
source "$crankstart_crate_dir/scripts/vars.sh" || exit $?

bindgen "$crankstart_crate_dir/crankstart-sys/wrapper.h" \
  --use-core \
  --ctypes-prefix ctypes \
  --with-derive-default \
  --whitelist-type PlaydateAPI \
  --whitelist-type PDSystemEvent \
  --whitelist-type LCDSolidColor \
  --whitelist-type PDEventHandler \
  --whitelist-var LCD_COLUMNS \
  --whitelist-var LCD_ROWS \
  --whitelist-var LCD_ROWSIZE \
  --rustified-enum SpriteCollisionResponseType \
  --bitfield-enum FileOptions \
  -- \
  -I"$PLAYDATE_C_API" \
  -DTARGET_EXTENSION > $crankstart_crate_dir/crankstart-sys/src/bindings_x86.rs


bindgen "$crankstart_crate_dir/crankstart-sys/wrapper.h" \
  --use-core \
  --ctypes-prefix ctypes \
  --with-derive-default \
  --whitelist-type PlaydateAPI \
  --whitelist-type PDSystemEvent \
  --whitelist-type LCDSolidColor \
  --whitelist-type PDEventHandler \
  --whitelist-var LCD_COLUMNS \
  --whitelist-var LCD_ROWS \
  --whitelist-var LCD_ROWSIZE \
  --rustified-enum SpriteCollisionResponseType \
  --bitfield-enum FileOptions \
  -- \
  -I"$PLAYDATE_C_API" \
  -I"/usr/local/playdate/gcc-arm-none-eabi-9-2019-q4-major/arm-none-eabi/include/" \
  -target thumbv7em-none-eabihf \
  -fshort-enums \
  -DTARGET_EXTENSION > $crankstart_crate_dir/crankstart-sys/src/bindings_arm.rs
