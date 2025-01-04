use atty::Stream;
use clap::{Parser, Subcommand};
use std::{collections::HashMap, fs::read_to_string, io::stdin};
use icalendar::{Calendar, CalendarComponent, Component};
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

    /// Calendar name; defaults to the first calendar name in the list of input files
    #[arg(long)]
    name: Option<String>,

    /// Calendar description; defaults to the first calendar description in the list of input files
    #[arg(long)]
    description: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Concatenate and merge multiple .ics files
    Cat {
        /// Input .ics files
        #[arg(required = false)]
        files: Vec<String>,
    },

    // /// Filter ou
    // Replace {
    //     /// Input .ics files
    //     #[arg(required = true)]
    //     files: Vec<String>,
    // },
}

/// Should the new_event replace the old_event?  Both have the same UID, and new_event was observed later.
fn must_replace(_new_event: &icalendar::Event, _old_event: &icalendar::Event) -> bool {
    true
}

/// Should this event be preserved? (Default filter)
fn ev_default_filter(_event: &icalendar::Event) -> bool {
    true
}

/// Should this event be transformed?  Return update, otherwise preserve
fn ev_default_transform(_event: &icalendar::Event) -> Option<icalendar::Event> {
    None
}

struct CalBuilder {
    components: Vec<CalendarComponent>,
    id_map: HashMap<String, usize>,
    name: Option<String>,
    description: Option<String>,
    timezone: Option<String>,
}

impl CalBuilder {
    fn new() -> Self {
	Self {
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

    fn process_stdin(&mut self) {
	let stdin = stdin();
	let mut input = String::new();
	for line in stdin.lines() {
            let line = line.expect("Stdin broken");
            // if line.trim().is_empty() {
	    // 	continue; // Ignore empty lines
            // }
            input.push_str(&line);
            input.push('\n');
	}
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
				must_replace(event, &old_event)
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

fn main() {
    let cli = Cli::parse();
    let mut output = CalBuilder::new();

    let ev_filter = &ev_default_filter;
    let ev_transform = &ev_default_transform;

    if !atty::is(Stream::Stdin) {
	output.process_stdin();
    }

    match &cli.command {
	Commands::Cat { files } => {
	    for file in files {
		let input = read_to_string(file).unwrap();
		output.process(&input);
	    }
	}
    }

    // Produce output

    let mut output_cal = output.empty_calendar();

    for component in output.components {
	let retain = if let CalendarComponent::Event(_) = component {
	    ev_filter(&component.as_event().unwrap())
	} else { true };

	if retain {
	    let preserve = match component {
		CalendarComponent::Event(ref ev) => {
		    match ev_transform(&ev) {
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

    println!("{}", output_cal);
}

