# Initial CLI contracts (ticket 01)

The command tree is deliberately domain-oriented (`device identity|state|capabilities|diagnose`); later slices should add sibling domains rather than place every operation at the root. `--target` (or `AWTRIX_URL`) is a required HTTP base URL. Optional `--username`/`--password` (or corresponding environment variables) enable Basic authentication. Requests have a finite `--timeout` in milliseconds (default 3000).

Machine output is compact JSON when `--json` is selected, independent of TTY detection. `--fields a,b` selects known top-level result fields; an unknown field is an argument error. Successful results go to stdout. Failures go to stderr and, with `--json`, use `{"error":{"code":"STABLE_CODE","message":"..."}}`. Exit codes: 0 success, 1 transport/general, 2 CLI/field selection, 3 authentication, 4 timeout, 5 HTTP, 6 incompatible device. Never include credentials in errors.

Device routes are the official AWTRIX NG `GET /api/v1/device`, `/api/v1/version`, `/api/v1/capabilities`, and `/api/v1/system` routes. `device diagnose` combines state, firmware version and capabilities; variant detection is from returned device/system identity and preserves TC002 as distinct from ESP32. Offline `describe [device]` advertises the ESP32 reference and needs no network. Capability names returned by the target are authoritative; no variant alone implies support.

Verified OpenAPI sources (retrieved 2026-10-07):

- https://blueforcer.github.io/awtrix-ng/esp32/api/openapi.yaml
- https://blueforcer.github.io/awtrix-ng/esp32-s3/api/openapi.yaml
- https://blueforcer.github.io/awtrix-ng/tc002/api/openapi.yaml

All three sources identify themselves as AWTRIX NG HTTP API OpenAPI 3.1.0, document the routes above and describe HTTP 401 for authenticated operations. Firmware versions and platform identity come from the live target, not a hardcoded compatibility claim. Unknown/unidentifiable hardware is reported as `unknown`.
