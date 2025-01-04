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

    /// Print a list of all properties used in at least one event
    Prop {
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
}

// --------------------------------------------------------------------------------
trait EventReplacementStrategy {
    /// Should the new_event replace the old_event?  Both have the same UID, and new_event was observed later.
    fn must_replace(&self, _new_event: &icalendar::Event, _old_event: &icalendar::Event) -> bool {
	true
    }
}

trait EventProcessor {
    /// Should this event be preserved? (Default filter)
    fn filter(&self, _event: &icalendar::Event) -> bool {
	true
    }
    /// Should this event be transformed?  Return update, otherwise preserve
    fn transform(&self, _event: &icalendar::Event) -> Option<icalendar::Event> {
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
    remove_set: HashSet<&'a String>,
}

impl<'a> RemovePropEventProcessor<'a> {
    fn new(remove_set: HashSet<&'a String>) -> Self {
	Self {
	    remove_set
	}
    }
}

impl<'a> EventProcessor for RemovePropEventProcessor<'a> {
    fn transform(&self, event: &icalendar::Event) -> Option<icalendar::Event> {
	let mut new_event = Event::new();
	for (k, v) in event.properties().iter() {
	    if !self.remove_set.contains(k) {
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
    fn transform(&self, event: &icalendar::Event) -> Option<icalendar::Event> {
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

struct CalBuilder<'a> {
    event_replacement_strategy: &'a dyn EventReplacementStrategy,
    components: Vec<CalendarComponent>,
    id_map: HashMap<String, usize>,
    name: Option<String>,
    description: Option<String>,
    timezone: Option<String>,
}

impl<'a> CalBuilder<'a> {
    fn new(event_replacement_strategy: &'a dyn EventReplacementStrategy) -> Self {
	Self {
	    event_replacement_strategy,
	    components: vec![],
	    id_map: HashMap::new(),
	    name: None,
	    description: None,
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

    fn calendar(self, event_processor: &dyn EventProcessor) -> Calendar {
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
	if input.len() > 0 {
	    let parsed_calendar: Calendar = input.parse().unwrap();

	    self.or_calendar(&parsed_calendar);

	    for component in &parsed_calendar.components {
		if let CalendarComponent::Event(event) = component {

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
		    }
		} else {
		    self.components.push(component.clone());
		}
	    }
	}
    }
}

// --------------------------------------------------------------------------------
// --------------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let mut output = CalBuilder::new(&DefaultEventReplacementStrategy{});
    let default_event_processor: &dyn EventProcessor = &DefaultEventProcessor{};

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

	Commands::RemoveProp { properties } => {
	    let mut properties_set = HashSet::new();
	    for prop in properties {
		properties_set.insert(prop);
	    }
	    let event_processor = RemovePropEventProcessor::new(properties_set);
	    // Produce output
	    cli.print_calendar(&output.calendar(&event_processor));
	}

	Commands::SetProp { property, value } => {
	    let event_processor = ReplacePropEventProcessor::new(property.clone(), value.clone());
	    // Produce output
	    cli.print_calendar(&output.calendar(&event_processor));
	}

	Commands::Prop { } => {
	    todo!("WIP");
	}
    }
}

