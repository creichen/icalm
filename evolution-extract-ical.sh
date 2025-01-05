#! /bin/bash

if [ x$1 == x ]; then
    echo "Usage: %0 ~/.cache/evolution/calendar/.../cache.db"
    echo ""
    echo "Translates an Evolution calendar cache into ical (ics) format."
    echo "LIMITATION: Does not automatically include all relevant time zones."
    exit 1
fi

# Include one default IANA time zone
cat <<EOF
BEGIN:VCALENDAR
VERSION:2.0
PRODID:ICALENDAR-RS
CALSCALE:GREGORIAN
BEGIN:VTIMEZONE
DTSTAMP:20250104T181459Z
TZID:Europe/Copenhagen
UID:af073073-a47e-4260-bb14-c6df7fd343fd
BEGIN:STANDARD
DTSTAMP:20250104T181459Z
DTSTART:20001029T040000
RRULE:FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10
TZNAME:CET
TZOFFSETFROM:+0200
TZOFFSETTO:+0100
UID:3a834cf7-e932-4239-91dc-e808067c8672
END:STANDARD
BEGIN:DAYLIGHT
DTSTAMP:20250104T181459Z
DTSTART:20000326T020000
RRULE:FREQ=YEARLY;BYDAY=-1SU;BYMONTH=3
TZNAME:CEST
TZOFFSETFROM:+0100
TZOFFSETTO:+0200
UID:938b3e51-29fd-494f-b8f5-1ff886993968
END:DAYLIGHT
END:VTIMEZONE
EOF

sqlite3 $1 -newline '' 'select zone from timezones'
sqlite3 $1 -newline '' 'select EcacheOBJ from EcacheObjects'

echo "END:VCALENDAR"
