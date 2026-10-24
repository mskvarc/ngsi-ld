# Authored fixtures

These fixtures are written by hand from the representation rules in ETSI GS CIM 009 v1.9.1. They cover the attribute kinds that the Annex C examples in [`../etsi/`](../etsi/) are too old to include.

They're licensed under MIT No Attribution (see [`LICENSE.md`](LICENSE.md)), which covers only this directory. You can copy them into your own test suite without keeping a notice.

## Vehicle

`vehicle-normalized.json`, `vehicle-concise.json` and `vehicle-simplified.json` hold the same entity (clause 4.5.1) in normalized, concise and simplified form. Clause 4.5.4 defines the simplified form, and each attribute kind's own clause defines its normalized and concise forms.

| Attribute | Kind | Clause |
|---|---|---|
| `modelName` | Property | 4.5.2 |
| `destination` | LanguageProperty | 4.5.18 |
| `fuelType` | VocabProperty | 4.5.20 |
| `axleLoads` | ListProperty with `valueType` and `unitCode` | 4.5.21 |
| `crew` | Relationship with several objects and an `objectType` | 4.5.3, 4.5.23 |
| `dockedAt` | Relationship with `observedAt` and a sub-Relationship | 4.5.3 |
| `stops` | ListRelationship | 4.5.22 |

## Notifications

`notification-normalized.json`, `notification-concise.json` and `notification-simplified.json` are notifications (clause 5.3.1) carrying the vehicle in each form. Each is what a subscriber receives when its subscription asks for that `format` (clause 5.8.6).

## Canonical form

Every file here is in the crate's canonical form, which `docs/design.md` defines. The golden tests depend on that: they write each document back out and compare the text, member order included.
