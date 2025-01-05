# icalm: ical (.ics) file merging and mangling tool

This tool processes `ics` (iCal) files:
- `cat`: concatenation (for duplicate events, report only the last occurrence)
- `remove-prop`: strip out blocklisted properties
- `keep-prop`: strip out properties unless passlisted
- `set-prop`: overwrite properties
- `tz-subst`: substitute timezone names
- `limit`: bound number of events in output

## Examples

Concatenation:
`icalm cat foo.ics bar.ics > out.ics`

Redacting event summaries:
`icalm -i foo.ics set-prop SUMMARY REDACTED`

Removing event locations:
`icalm -i foo.ics remove-prop LOCATION`

Piplining:
`icalm cat foo.ics bar.ics | icalm remove-prop LOCATION | icalm -o out.ics set-prop SUMMARY REDCATED`


## Motivation

`ics` (icalendar, iCal) files seem to be the only unversal (albeit
read-only) calendar view supported by all major calendar providers.

However, `ics` files aren't always in the ideal format for export to
e.g. Google Calendar or Outlook:
- Calendars may be stored in multiple `ics` files (e.g., by
  `radicale`), while Google Calendar and Outlook will import only a
  single `is` link at a time
- Calendars may store information that we don't want to make public
  (e.g., phone numbers or Zoom links).

`icalm` aims to address these challenges by offering a fast interface
for merging and processing

## Status
- `icalm` supports the (presumably) most common use cases
- The `icalm` implementation is small and provides an `EventProcessor`
  interface that should make it easy to facilitate many other use
  cases (filtering out events, conditional event transformation, ...)
