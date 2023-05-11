//! Common Library

use chrono::{DateTime, TimeZone, Timelike};

#[derive(Debug, Clone, Copy)]
pub enum PartOfDay {
    Morning,
    Afternoon,
}
impl PartOfDay {
    pub fn begin(&self) -> PointOfDay {
        match self {
            PartOfDay::Morning => PointOfDay::Dawn,
            PartOfDay::Afternoon => PointOfDay::Noon,
        }
    }

    pub fn end(&self) -> PointOfDay {
        match self {
            PartOfDay::Morning => PointOfDay::Noon,
            PartOfDay::Afternoon => PointOfDay::Dusk,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            PartOfDay::Morning => "Morning".to_string(),
            PartOfDay::Afternoon => "Afternoon".to_string(),
        }
    }
}

pub const ALL_DAY: [PartOfDay; 2] = [PartOfDay::Morning, PartOfDay::Afternoon];

#[derive(Debug)]
pub enum PointOfDay {
    Dawn,
    Noon,
    Dusk,
}
impl PointOfDay {
    pub fn value(&self) -> usize {
        match self {
            PointOfDay::Dawn => 6,
            PointOfDay::Noon => 12,
            PointOfDay::Dusk => 18,
        }
    }
    pub fn datetime<T: TimeZone>(&self, datetime: DateTime<T>) -> DateTime<T> {
        let hour = self.value();
        datetime
            .with_hour(hour as u32)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
    }
}
