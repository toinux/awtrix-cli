# AWTRIX NG feature coverage

What `awtrix-cli` covers of the AWTRIX NG device API, and what it does not. The
goal is to spot gaps and newly documented AWTRIX NG features quickly — not to
restate the device documentation, which stays authoritative.

## Baseline

| | |
| --- | --- |
| Last reviewed AWTRIX NG commit | `fb09bc5c8f7e` (2026-10-10, version 1.2.2) |
| AWTRIX NG docs version | **1.2.2** (released 2026-10-06) |
| Contracts last reviewed | **2026-10-15** |
| Documented routes | 62 — 23 covered, 39 not covered |
| Documented pages | 110 Markdown pages |

The documentation source of truth is the [Blueforcer/awtrix-ng](https://github.com/Blueforcer/awtrix-ng)
repository, not the published site: `docs/` holds one Markdown tree plus one
`docs/api/openapi.yaml` whose path-level `x-only: [...]` markers select the clock
variants. `tools/docs/build_site.py` renders the four published sites (chooser,
ESP32, ESP32-S3, TC002) from it. Route and page comparisons therefore run against
git, and the reviewed commit above is the only baseline.

## Re-verify

The `awtrix-coverage` project skill fetches the repository and diffs it against
the reviewed commit:

```sh
.opencode/skills/awtrix-coverage/scripts/awtrix-coverage.sh
```

It reports the version change, new/changed/removed routes and pages since that
commit, checks the covered list against `src/`, prints the coverage table, and
exits non-zero on any drift. `NEW route:`, `changed route:`, a `page:` line and
`docs version:` are the candidate list for this document. After reviewing them,
record the new state:

```sh
.opencode/skills/awtrix-coverage/scripts/awtrix-coverage.sh --snapshot
```

`--snapshot` updates the reviewed commit in this file. Run it only after the
review.

When a command starts or stops calling a route, edit
`.opencode/skills/awtrix-coverage/data/covered.txt`; the script fails if a listed
route no longer appears in `src/`.

## Gaps at a glance

| Theme | Uncovered routes | Targets |
| --- | --- | --- |
| Audio: melodies, songs, MP3, radio, play/stop | `/api/v1/audio*` (10 routes) | all; `stations` S3+TC002; `clip` TC002 |
| Script-owned sounds | `/api/v1/apps/script/{name}/sounds[/{sound}]` | all |
| Cross-script state | `/api/v1/scripts/shared` | all |
| Manual rotation | `/api/v1/apps/next`, `/api/v1/apps/previous` | all |
| Built-in app settings | `/api/v1/apps/builtin/{name}/config` | all |
| Corner indicators | `/api/v1/indicators/{id}` | all |
| Mood light | `/api/v1/display/moodlight` | all |
| Icon management | `/api/v1/icons/origins`, `/api/v1/icons/rename` | all |
| Wi-Fi setup | `/api/v1/system/wifi-scan` | all |
| Destructive admin | `factory-reset`, `settings/reset`, `restore`, `/update` | all |
| Deep sleep | `/api/v1/device/sleep` | ESP32, ESP32-S3 |
| Script sign-in (OAuth) | `/api/v1/oauth*` (4 routes) | TC002 |
| Bluetooth gamepads | `/api/v1/gamepad*` (5 routes) | TC002 |
| MQTT TLS trust | `/api/v1/mqtt/tls[/ca]` | TC002 |
| Home Assistant Voice | `/api/v1/voice` | TC002 |

## Routes

`CLI` = the route is called by `awtrix-cli`. `-` = the route is absent on that
target. Targets: **E** = ESP32, **S** = ESP32-S3, **T** = TC002.

### Device

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/device` | get | get | get | yes |
| `/api/v1/device/reboot` | post | post | post | yes |
| `/api/v1/device/sleep` | post | post | - | no |
| `/api/v1/device/factory-reset` | post | post | post | no |
| `/api/v1/version` | get | get | get | yes |
| `/version` (plain text) | get | get | get | no |
| `/update` (firmware) | post | post | post | no |
| `/api/v1/restore` (backup) | post | post | post | no |
| `/api/v1/capabilities` | get | get | get | yes |

### Display, settings, system

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/display` | get,patch | get,patch | get,patch | yes |
| `/api/v1/display/screen` | get | get | get | yes |
| `/api/v1/display/moodlight` | delete,put | delete,put | delete,put | no |
| `/api/v1/settings` | get,patch | get,patch | get,patch | yes (2 keys only) |
| `/api/v1/settings/reset` | post | post | post | no |
| `/api/v1/system` | get,put | get,put | get,put | get only |
| `/api/v1/system/wifi-scan` | get | get | get | no |

### Apps and rotation

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/apps` | get | get | get | yes |
| `/api/v1/apps/active` | put | put | put | yes |
| `/api/v1/apps/next` | post | post | post | no |
| `/api/v1/apps/previous` | post | post | post | no |
| `/api/v1/apps/order` | put | put | put | yes |
| `/api/v1/apps/{name}` | delete | delete | delete | yes |
| `/api/v1/apps/{name}/enabled` | put | put | put | yes |
| `/api/v1/apps/pushed/{name}` | put | put | put | yes |
| `/api/v1/apps/{name}/config` | get,patch | get,patch | get,patch | yes |
| `/api/v1/apps/{name}/data` | get,patch | get,patch | get,patch | get only |
| `/api/v1/apps/builtin/{name}/config` | get,patch | get,patch | get,patch | no |

### Notifications and indicators

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/notifications` | post | post | post | yes |
| `/api/v1/notifications/active` | delete | delete | delete | yes |
| `/api/v1/notifications/{name}` | delete | delete | delete | yes |
| `/api/v1/indicators/{id}` | delete,put | delete,put | delete,put | no |

### Scripts, modules, files

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/apps/script/{name}` | get,put | get,put | get,put | yes |
| `/api/v1/apps/script-update/{name}` | put | put | put | yes |
| `/api/v1/apps/script/{name}/sounds` | delete,get,post | delete,get,post | delete,get,post | no |
| `/api/v1/apps/script/{name}/sounds/{sound}` | delete | delete | delete | no |
| `/api/v1/scripts/shared` | get | get | get | no |
| `/api/v1/files` | delete,get,post | delete,get,post | delete,get,post | yes |
| `/api/v1/icons/origins` | delete,get,put | delete,get,put | delete,get,put | no |
| `/api/v1/icons/rename` | post | post | post | no |
| `/api/v1/logs` | get | get | get | yes |

### Audio

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/audio` | get | get | get | no |
| `/api/v1/audio/play` | post | post | post | no |
| `/api/v1/audio/stop` | post | post | post | no |
| `/api/v1/audio/melodies` | get | get | get | no |
| `/api/v1/audio/melodies/{name}` | delete,put | delete,put | delete,put | no |
| `/api/v1/audio/mp3` | get,post | get,post | get,post | no |
| `/api/v1/audio/mp3/{name}` | delete | delete | delete | no |
| `/api/v1/audio/mp3/rename` | post | post | post | no |
| `/api/v1/audio/clip` | - | - | post | no |
| `/api/v1/audio/stations` | - | get,put | get,put | no |

### TC002 only

| Route | E | S | T | CLI |
| --- | --- | --- | --- | --- |
| `/api/v1/oauth` | - | - | get | no |
| `/api/v1/oauth/{name}` | - | - | delete,get,post | no |
| `/api/v1/oauth/{name}/start` | - | - | post | no |
| `/api/v1/oauth/{name}/code` | - | - | post | no |
| `/api/v1/gamepad` | - | - | get | no |
| `/api/v1/gamepad/pair` | - | - | post | no |
| `/api/v1/gamepad/{id}` | - | - | delete | no |
| `/api/v1/gamepad/remote` | - | - | post | no |
| `/api/v1/gamepad/remote/{session}` | - | - | delete | no |
| `/api/v1/mqtt/tls` | - | - | get | no |
| `/api/v1/mqtt/tls/ca` | - | - | delete,put | no |
| `/api/v1/voice` | - | - | get,post | no |

## Partially covered schemas

### `Settings` — 2 of 46 keys

`settings patch` accepts only the two keys below; every other key is rejected
locally (`unsupported setting field`) and never forwarded.

- **Covered:** `brightness`, `autoBrightness`
- Not covered: `autoTransition`, `textColor`, `transitionEffect`,
  `transitionDirection`, `transitionDurationMs`, `appDurationMs`, `timeMode`,
  `clockFace`, `calendarHeaderColor`, `calendarTextColor`, `calendarBodyColor`,
  `calendarAnimation`, `time24h`, `timeLeadingZero`, `timeShowSeconds`,
  `timeShowAmPm`, `timeSeparatorMode`, `dateOrder`, `dateSeparator`,
  `dateYearMode`, `dateShowWeekday`, `dateMonthNames`, `useCelsius`,
  `blockNavigation`, `bootSound`, `uppercase`, `enlargeApps`, `weekdayBar`,
  `dateWeekdayBar`, `timeColor`, `dateColor`, `humidityColor`,
  `temperatureColor`, `batteryColor`, `scroll`, `volume`, `radioVolume`,
  `appVolume`, `alertVolume`, `musicSource`, `saturation`, `gamma`,
  `colorCorrection`, `colorTint`

This hides the 1.2.2 headline features (clock faces, sound mixer) from the CLI.

### `SystemConfig` — read-only

`settings system-get` reads all 37 keys (secrets redacted) but nothing writes
them. Not covered as writes: `wifiSsid`, `wifiPass`, `netStatic`, `ip`,
`gateway`, `subnet`, `dns1`, `dns2`, `mqttEnabled`, `mqttHost`, `mqttPort`,
`mqttUser`, `mqttPass`, `mqttPrefix`, `mqttTls`, `mqttTlsPin`, `haDiscovery`,
`haPrefix`, `ntpServer`, `tz`, `tzName`, `hostname`, `authEnabled`, `authUser`,
`authPass`, `lowBatteryThreshold`, `swapButtons`, `buttonCallback`,
`mirrorShare`, `mirrorShareApps`, `mirrorShareNotifications`, `mirrorFrom`,
`mirrorFromApps`, `mirrorFromNotifications`, `statsInterval`, `debugMode`,
`scriptingEnabled`. Mirroring lives here, so it is unreachable through the CLI.

### `Display` — patch subset

`display patch` accepts `power`, `overlay`, `overlaySettings` only. `brightness`
is read-only on this route, and `moodlight` is a separate uncovered route.

### Pushed-app payload

35 top-level keys are type-checked locally; unknown keys need `--raw`. The
`layout` key (`NativeLayout`, the 1.2.2 layouts feature) is **not** in the typed
set, so layouts only pass through `--raw` without local validation.
