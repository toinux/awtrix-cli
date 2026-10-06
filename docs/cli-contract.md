# Initial CLI contracts (ticket 01)

The command tree is deliberately domain-oriented (`device identity|state|capabilities|diagnose`); later slices should add sibling domains rather than place every operation at the root. `--target` (or `AWTRIX_URL`) is a required HTTP base URL. Optional `--username`/`--password` (or corresponding environment variables) enable Basic authentication. Requests have a finite `--timeout` in milliseconds (default 3000).

Machine output is compact JSON when `--json` is selected, independent of TTY detection. `--fields a,b` selects known top-level result fields; an unknown field is an argument error. Successful results go to stdout. Under `--json`, structured failures go to stdout as `{"error":{"code":"STABLE_CODE","message":"..."}}`, while a concise diagnostic also goes to stderr. CLI parse failures additionally emit clap's usage diagnostic to stderr. Exit codes: 0 success, 1 transport/general, 2 CLI/field selection, 3 authentication, 4 timeout, 5 HTTP, 6 incompatible device. Never include credentials or raw server response bodies in errors.

Device routes are the official AWTRIX NG `GET /api/v1/device`, `/api/v1/version`, `/api/v1/capabilities`, and `/api/v1/system` routes. Current identity uses `boardType` and `soc` from `/api/v1/device`: `awtrixng` + `esp32` means ESP32, `awtrixng` + `esp32s3` means ESP32-S3, and `tc002` means TC002. Other strings do not imply a variant. Identity and diagnosis require device state; no unrelated `/system` request is made. Offline `describe [topic]` advertises the ESP32 reference; supplying `--target` explicitly refines it with connected capabilities and propagates request failures. Capability names returned by the target are authoritative; no variant alone implies support.

Verified OpenAPI sources (retrieved 2026-10-07):

- https://blueforcer.github.io/awtrix-ng/esp32/api/openapi.yaml
- https://blueforcer.github.io/awtrix-ng/esp32-s3/api/openapi.yaml
- https://blueforcer.github.io/awtrix-ng/tc002/api/openapi.yaml

All three sources identify themselves as AWTRIX NG HTTP API OpenAPI 3.1.0 and define `DeviceState` identity constants: ESP32 has `boardType: awtrixng` / `soc: esp32`, ESP32-S3 has `boardType: awtrixng` / `soc: esp32s3`, and TC002 has `boardType: tc002`. They document the routes above and HTTP 401 for authenticated operations. Firmware versions and platform identity come from the live target, not a hardcoded compatibility claim. Unknown/unidentifiable hardware is reported as `unknown`.
