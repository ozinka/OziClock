//! Application-layer composition points for OziClock use cases.

pub mod calendar;

use oziclock_domain::{Clock, ClockCollection};

/// Typed clock-editing intents issued by presentation adapters.
pub enum ClockCommand {
    Add(Clock),
    Remove { index: usize },
    Move { from: usize, to: usize },
    SetMain { index: usize },
}

/// Applies one clock use case while preserving domain invariants.
pub fn execute_clock_command(clocks: &mut Vec<Clock>, command: ClockCommand) -> bool {
    if clocks.is_empty() {
        return false;
    }

    let current = std::mem::take(clocks);
    let mut collection = ClockCollection::new(current);
    let changed = match command {
        ClockCommand::Add(clock) => {
            collection.add(clock);
            true
        }
        ClockCommand::Remove { index } => collection.remove(index),
        ClockCommand::Move { from, to } => collection.move_to(from, to),
        ClockCommand::SetMain { index } => collection.set_main(index),
    };
    *clocks = collection.into_vec();
    changed
}

/// Returns the product name owned by the domain layer.
pub fn application_name() -> &'static str {
    oziclock_domain::PRODUCT_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(label: &str, is_main: bool) -> Clock {
        Clock {
            label: label.into(),
            time_zone: "UTC".into(),
            color: "#FFFFFF".into(),
            is_main,
        }
    }

    #[test]
    fn remove_command_cannot_empty_the_collection() {
        let mut clocks = vec![clock("UTC", true)];
        assert!(!execute_clock_command(
            &mut clocks,
            ClockCommand::Remove { index: 0 }
        ));
        assert_eq!(clocks.len(), 1);
    }

    #[test]
    fn set_main_command_keeps_exactly_one_main_clock() {
        let mut clocks = vec![clock("A", true), clock("B", false)];
        assert!(execute_clock_command(
            &mut clocks,
            ClockCommand::SetMain { index: 1 }
        ));
        assert!(!clocks[0].is_main);
        assert!(clocks[1].is_main);
    }
}
