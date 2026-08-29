/// A configured world clock.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct Clock {
    pub label: String,
    pub time_zone: String,
    pub color: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_main: bool,
}

/// Domain-owned clock collection and its invariants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockCollection {
    clocks: Vec<Clock>,
}

impl ClockCollection {
    pub fn new(mut clocks: Vec<Clock>) -> Self {
        assert!(!clocks.is_empty(), "a clock collection cannot be empty");
        normalize_main_clock(&mut clocks);
        Self { clocks }
    }

    pub fn as_slice(&self) -> &[Clock] {
        &self.clocks
    }

    pub fn add(&mut self, clock: Clock) -> usize {
        self.clocks.push(clock);
        self.clocks.len() - 1
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if self.clocks.len() == 1 || index >= self.clocks.len() {
            return false;
        }
        let removed_main = self.clocks.remove(index).is_main;
        if removed_main {
            self.clocks[0].is_main = true;
        }
        true
    }

    pub fn move_to(&mut self, from: usize, to: usize) -> bool {
        if from >= self.clocks.len() || to >= self.clocks.len() || from == to {
            return false;
        }
        self.clocks.swap(from, to);
        true
    }

    pub fn set_main(&mut self, index: usize) -> bool {
        if index >= self.clocks.len() {
            return false;
        }
        for (current, clock) in self.clocks.iter_mut().enumerate() {
            clock.is_main = current == index;
        }
        true
    }

    pub fn into_vec(self) -> Vec<Clock> {
        self.clocks
    }
}

fn normalize_main_clock(clocks: &mut [Clock]) {
    let main_index = clocks.iter().position(|clock| clock.is_main).unwrap_or(0);
    for (index, clock) in clocks.iter_mut().enumerate() {
        clock.is_main = index == main_index;
    }
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
    fn normalizes_collection_to_exactly_one_main_clock() {
        let clocks = ClockCollection::new(vec![clock("A", true), clock("B", true)]);
        assert!(clocks.as_slice()[0].is_main);
        assert!(!clocks.as_slice()[1].is_main);
    }

    #[test]
    fn cannot_remove_last_clock() {
        let mut clocks = ClockCollection::new(vec![clock("A", true)]);
        assert!(!clocks.remove(0));
    }

    #[test]
    fn removing_main_clock_selects_first_remaining_clock() {
        let mut clocks = ClockCollection::new(vec![clock("A", true), clock("B", false)]);
        assert!(clocks.remove(0));
        assert!(clocks.as_slice()[0].is_main);
    }
}
