use clap::{Parser, Subcommand};
use std::{collections::HashMap, fs::read_to_string};
use icalendar::{Calendar, CalendarComponent, Component};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = "A command-line tool for processing iCalendar (.ics) files"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Concatenate and merge multiple .ics files
    Cat {
        /// Input .ics files
        #[arg(required = true)]
        files: Vec<String>,

	/// Calendar name; defaults to the first calendar name in the list of input files
        #[arg(long)]
        name: Option<String>,

    	/// Calendar description; defaults to the first calendar description in the list of input files
        #[arg(long)]
        description: Option<String>,
    },
}

fn must_replace(_new_event: &icalendar::Event, _old_event: &icalendar::Event) -> bool {
    true
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
	Commands::Cat { files, name, description } => {
	    let mut output: Vec<CalendarComponent> = vec![];
	    let mut id_map = HashMap::new();
	    let mut cal_name: Option<String> = name.clone();
	    let mut cal_description: Option<String> = description.clone();
	    let mut cal_timezone = None;

	    for file in files {
		let contents = read_to_string(file).unwrap();

		let parsed_calendar: Calendar = contents.parse().unwrap();

		cal_name = cal_name.take().or(parsed_calendar.get_name().map(|s| s.to_string()));
		cal_description = cal_description.take().or(parsed_calendar.get_description().map(|s| s.to_string()));
		cal_timezone = cal_timezone.take().or(parsed_calendar.get_timezone().map(|s| s.to_string()));

		for component in &parsed_calendar.components {
		    if let CalendarComponent::Event(event) = component {
			if let Some(uid) = event.get_uid() {
			    let uid = uid.to_string();
			    if let Some(&index) = id_map.get(&uid) {
				// Already saw this UID?
				let refcell = &mut output[index];

				let to_replace = if let CalendarComponent::Event(old_event) = refcell {
				    must_replace(event, &old_event)
				} else { false };

				if to_replace {
				    *refcell = component.clone();
				}
			    } else {
				// Fresh UID
				id_map.insert(uid, output.len());
				output.push(component.clone());
			    }
			} else {
			    eprintln!("Calendar event without UID; skipping");
			}
		    } else {
			output.push(component.clone());
		    }

		    // if let CalendarComponent::Event(event) = component {
		    // 	output.push(event.clone());
		    // 	// if let Some(summary) = event.get_summary() {
		    // 	//     println!("Event: {}", summary);
		    // 	// }
		    // }
		}
	    }

	    let mut output_cal = Calendar::new();
	    for component in output {
		output_cal.push(component);
	    }

	    if let Some(name) = cal_name {
		output_cal.name(&name);
	    }

	    if let Some(description) = cal_description {
		output_cal.description(&description);
	    }

	    if let Some(timezone) = cal_timezone {
		output_cal.timezone(&timezone);
	    }

	    println!("{}", output_cal);
	}
    }
}

