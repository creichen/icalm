# icalm: ical file merging and mangling tool

This tool processes `ical` files:
- `cat`: concatenation (incl. replacing duplicate events)
- `remove-prop`: strip out properties
- `set-prop`: overwrite properties

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

`ical` (icalendar, iCal) files seem to be the only unversal (albeit
read-only) calendar view supported by all major calendar providers.

However, `ical` files aren't always in the ideal format for export to
e.g. Google Calendar or Outlook:
- Calendars may be stored in multiple `ical` files (e.g., by
  `radicale`), while Google Calendar and Outlook will import only a
  single `ical` link at a time
- Calendars may store information that we don't want to make public
  (e.g., phone numbers or Zoom links).

`icalm` aims to address these challenges by offering a fast interface
for merging and processing

## Status
- `icalm` supports the (presumably) most common use cases
- The `icalm` implementation is small and provides an `EventProcessor`
  interface that should make it easy to facilitate many other use
  cases (filtering out events, conditional event transformation, ...)
