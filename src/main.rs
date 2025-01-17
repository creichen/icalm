use atty::Stream;
use clap::{Parser, Subcommand};
use std::{collections::{HashMap, HashSet}, fs::{read_to_string, File}, io::{self, Write}};
use icalendar::{Calendar, CalendarComponent, Component, Event};
//use colored::Colorize;

#[derive(Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = "A command-line tool for processing iCalendar (.ics) files"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Input file
    #[arg(short, long)]
    input: Option<String>,

    /// Output file
    #[arg(short, long)]
    output: Option<String>,

    /// Calendar name; defaults to the first calendar name in the list of input files
    #[arg(long)]
    name: Option<String>,

    /// Calendar description; defaults to the first calendar description in the list of input files
    #[arg(long)]
    description: Option<String>,
}

impl Cli {
    fn print_calendar(&self, output_cal: &Calendar) {
	if let Some(ref output_filename) = self.output {
	    println!("Redirection");
	    let mut file = File::create(output_filename).unwrap();
	    writeln!(file, "{}", output_cal).unwrap();
	} else {
	    println!("{}", output_cal);
	}
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Concatenate and merge multiple .ics files
    Cat {
        /// Input .ics files
        #[arg(required = false)]
        files: Vec<String>,
    },

    /// Remove the specified properties (SUMMARY, LOCATION, STATUS, ...) from all events
    RemoveProp {
	/// Properties to remove
        #[arg(required = true)]
        properties: Vec<String>,
    },

    /// From all events, remove all properties EXCEPT for the specified properties (SUMMARY, LOCATION, STATUS, ...)
    KeepProp {
	/// Properties to remove
        #[arg(required = true)]
        properties: Vec<String>,
    },

    /// Print a list of all properties used in at least one event
    Prop {
    },

    /// Limit the number of events to report
    Limit {
	/// Maximal number of events
        #[arg(required = true)]
        max: usize,
    },

    /// Replace the value of one property by a constant string
    SetProp {
        /// Property to replace (e.g., "SUMMARY")
        #[arg(required = true)]
        property: String,

        /// Value to substitute for this property
        #[arg(required = true)]
        value: String,
    },

    /// Replace the name of one time zone by another WITHOUT altering the time.  This is intended for fixing broken ical files.
    TzSubst {
        /// Original zone (e.g., "Greenwich")
        #[arg(required = true)]
        from_tz: String,

        /// Substitute (e.g., "UTC")
        #[arg(required = true)]
        to_tz: String,
    },
}

// --------------------------------------------------------------------------------
trait EventReplacementStrategy {
    /// Should the new_event replace the old_event?  Both have the same UID, and new_event was observed later.
    fn must_replace(&mut self, _new_event: &icalendar::Event, _old_event: &icalendar::Event) -> bool {
	true
    }
}

trait EventProcessor {
    /// Should this event be preserved? (Default filter)
    fn filter(&mut self, _event: &icalendar::Event) -> bool {
	true
    }
    /// Should this event be transformed?  Return update, otherwise preserve
    fn transform(&mut self, _event: &icalendar::Event) -> Option<icalendar::Event> {
	None
    }
}

// --------------------------------------------------------------------------------

struct DefaultEventReplacementStrategy {}
impl EventReplacementStrategy for  DefaultEventReplacementStrategy {}

struct DefaultEventProcessor {}
impl EventProcessor for  DefaultEventProcessor {}

// --------------------------------------------------------------------------------

struct RemovePropEventProcessor<'a> {
    properties_set: HashSet<&'a String>,
    keep: bool,  // If true, keep ONLY the elements contained in the set
}

impl<'a> RemovePropEventProcessor<'a> {
    fn new(properties: &'a [String], keep: bool) -> Self {
	let mut properties_set = HashSet::new();
	for prop in properties {
	    properties_set.insert(prop);
	}

	Self {
	    keep,
	    properties_set,
	}
    }
}

impl<'a> EventProcessor for RemovePropEventProcessor<'a> {
    fn transform(&mut self, event: &icalendar::Event) -> Option<icalendar::Event> {
	let mut new_event = Event::new();
	for (k, v) in event.properties().iter() {
	    if self.keep == self.properties_set.contains(k) {
		new_event.append_property(v.clone());
	    }
	}
	return Some(new_event);
    }
}

// --------------------------------------------------------------------------------

struct ReplacePropEventProcessor {
    property: String,
    value: String,
}

impl ReplacePropEventProcessor {
    fn new(property: String, value: String) -> Self {
	Self {
	    property,
	    value,
	}
    }
}

impl EventProcessor for ReplacePropEventProcessor {
    fn transform(&mut self, event: &icalendar::Event) -> Option<icalendar::Event> {
	let mut new_event = Event::new();
	for (k, v) in event.properties().iter() {
	    if *k != self.property {
		new_event.append_property(v.clone());
	    }
	}
	new_event.add_property(&self.property, &self.value);
	return Some(new_event);
    }
}

// --------------------------------------------------------------------------------

// Substitute time zone name in events
struct TzSubstEventProcessor {
    from_tz: String,
    to_tz: String,
}

impl TzSubstEventProcessor {
    fn new(from_tz: String, to_tz: String) -> Self {
	Self {
	    from_tz,
	    to_tz,
	}
    }
}

impl EventProcessor for TzSubstEventProcessor {
    fn transform(&mut self, event: &icalendar::Event) -> Option<icalendar::Event> {
	let mut new_event = Event::new();
	for (_, v) in event.properties().iter() {
	    let to_replace =
		if let Some(tzid) = v.params().get("TZID") {
		    if tzid.value() == self.from_tz {
			true
		    } else { false }
		} else { false };

	    if to_replace {
		let params = v.params().iter().map(
		    |(k, p)| if k == "TZID" { icalendar::Parameter::new(k, &self.to_tz) } else { p.clone() });
		let mut new_prop = icalendar::Property::new(v.key(), v.value());
		for param in params {
		    new_prop.append_parameter(param);
		}
		new_event.append_property(new_prop);
	    } else {
		new_event.append_property(v.clone());
	    }
	}
	return Some(new_event);
    }
}

// --------------------------------------------------------------------------------

struct LimitEventProcessor {
    remaining: usize,
}

impl LimitEventProcessor {
    fn new(remaining: usize) -> Self {
	Self {
	    remaining,
	}
    }
}

impl EventProcessor for LimitEventProcessor {
    fn filter(&mut self, _event: &icalendar::Event) -> bool {
	if self.remaining > 0 {
	    self.remaining -= 1;
	    return true;
	}
	return false;
    }
}

// --------------------------------------------------------------------------------

struct CalBuilder<'a> {
    event_replacement_strategy: &'a mut dyn EventReplacementStrategy,
    components: Vec<CalendarComponent>,
    id_map: HashMap<String, usize>,
    name: Option<String>,
    description: Option<String>,
    timezone: Option<String>,
}

impl<'a> CalBuilder<'a> {
    fn new(event_replacement_strategy: &'a mut dyn EventReplacementStrategy, cli: &Cli) -> Self {
	Self {
	    event_replacement_strategy,
	    components: vec![],
	    id_map: HashMap::new(),
	    name: cli.name.clone(),
	    description: cli.description.clone(),
	    timezone: None,
	}
    }

    fn or_calendar(&mut self, calendar: &Calendar) {
	self.name = self.name.take().or(calendar.get_name().map(|s| s.to_string()));
	self.description = self.description.take().or(calendar.get_description().map(|s| s.to_string()));
	self.timezone = self.timezone.take().or(calendar.get_timezone().map(|s| s.to_string()));
    }

    fn empty_calendar(&self) -> Calendar {
	let mut output_cal = Calendar::new();

	if let Some(ref name) = self.name {
	    output_cal.name(&name);
	}

	if let Some(ref description) = self.description {
	    output_cal.description(&description);
	}

	if let Some(ref timezone) = self.timezone {
	    output_cal.timezone(&timezone);
	}
	return output_cal;
    }

    fn calendar(self, event_processor: &mut dyn EventProcessor) -> Calendar {
	let mut output_cal = self.empty_calendar();

	for component in self.components {
	    let retain = if let CalendarComponent::Event(_) = component {
		event_processor.filter(&component.as_event().unwrap())
	    } else { true };

	    if retain {
		let preserve = match component {
		    CalendarComponent::Event(ref ev) => {
			match event_processor.transform(&ev) {
			    None     => true,
			    Some(ev) => { output_cal.push(CalendarComponent::Event(ev));
					  false},
			}
		    },
		    _ => true,
		};
		if preserve {
		    output_cal.push(component);
		}
	    }
	}
	return output_cal;
    }

    fn process_stdin(&mut self) {
	let input = io::read_to_string(io::stdin()).unwrap();
	self.process(&input);
    }

    fn process_file(&mut self, filename: &str) {
	let input = read_to_string(filename).unwrap();
	self.process(&input);
    }

    fn process(&mut self, input: &str) {
	// For removing duplicate TZIDs
	let mut tzid_set = HashSet::new();

	if input.len() > 0 {
	    let parsed_calendar: Calendar = input.parse().unwrap();

	    self.or_calendar(&parsed_calendar);

	    for component in &parsed_calendar.components {
		match component {
		    CalendarComponent::Event(event) => {
			if let Some(uid) = event.get_uid() {
			    let uid = uid.to_string();
			    if let Some(&index) = self.id_map.get(&uid) {
				// Already saw this UID?
				let refcell = &mut self.components[index];

				let to_replace = if let CalendarComponent::Event(old_event) = refcell {
				    self.event_replacement_strategy.must_replace(event, &old_event)
				} else { false };

				if to_replace {
				    *refcell = component.clone();
				}
			    } else {
				// Fresh UID
				self.id_map.insert(uid, self.components.len());
				self.components.push(component.clone());
			    }
			} else {
			    eprintln!("Calendar event without UID; skipping");
			}},
		    CalendarComponent::Other(other) => {
			// Remove duplicate TZIDs
			let preserve: bool = if other.component_kind() == "VTIMEZONE" {
			    //eprintln!("{:?}", other.property_value("TZID"));
			    if let Some(tzid) = other.property_value("TZID") {
				if tzid_set.contains(tzid) {
				    false
				} else {
				    tzid_set.insert(tzid);
				    true
				}
			    } else { true }
			} else { true };
			if preserve {
			    self.components.push(component.clone());
			}
		    },
		    _ => {
			self.components.push(component.clone());
		    }
		}
	    }
	}
    }
}

// --------------------------------------------------------------------------------
// --------------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let mut default_replacement_strategy = DefaultEventReplacementStrategy{};
    let mut output = CalBuilder::new(&mut default_replacement_strategy, &cli);
    let mut default_event_processor_data = DefaultEventProcessor{};
    let default_event_processor: &mut dyn EventProcessor = &mut default_event_processor_data;

    if let Some(ref input_file) = cli.input {
	output.process_file(input_file);
    }

    if !atty::is(Stream::Stdin) {
	output.process_stdin();
    }

    match &cli.command {
	Commands::Cat { files } => {
	    for file in files {
		output.process_file(&file);
	    }
	    // Produce output
	    cli.print_calendar(&output.calendar(default_event_processor));
	}

	Commands::KeepProp { properties } => {
	    let mut event_processor = RemovePropEventProcessor::new(properties, true);
	    // Produce output
	    cli.print_calendar(&output.calendar(&mut event_processor));
	}

	Commands::RemoveProp { properties } => {
	    let mut event_processor = RemovePropEventProcessor::new(properties, false);
	    // Produce output
	    cli.print_calendar(&output.calendar(&mut event_processor));
	}

	Commands::SetProp { property, value } => {
	    let mut event_processor = ReplacePropEventProcessor::new(property.clone(), value.clone());
	    // Produce output
	    cli.print_calendar(&output.calendar(&mut event_processor));
	}

	Commands::TzSubst { from_tz, to_tz } => {
	    let mut event_processor = TzSubstEventProcessor::new(from_tz.clone(), to_tz.clone());
	    // Produce output
	    cli.print_calendar(&output.calendar(&mut event_processor));
	}

	Commands::Prop { } => {
	    // Produce output
	    let mut properties_set = HashSet::new();
	    for component in output.components {
		if let CalendarComponent::Event(event) = component {
		    for prop in event.properties().keys() {
			if !properties_set.contains(prop) {
			    println!("{}", prop);
			    properties_set.insert(prop.clone());
			}
		    }
		}
	    }
	}

	Commands::Limit { max } => {
	    let mut event_processor = LimitEventProcessor::new(*max);
	    // Produce output
	    cli.print_calendar(&output.calendar(&mut event_processor));
	}

    }
}

