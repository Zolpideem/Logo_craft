use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const DAY: i64 = 86_400;
const WEEK: i64 = 7 * DAY;

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs() as i64)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WeeklyCam {
    pub days: u8,
    pub start_minute: u16,
    pub end_minute: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recurrence {
    Once,
    Monthly,
    #[default]
    Yearly,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub utc_offset: i16,
    pub pulse: bool,
    pub cams: [WeeklyCam; 3],
    pub recurrence: Recurrence,
    pub start_date: [u16; 3],
    pub end_date: [u16; 3],
    pub latitude: f64,
    pub longitude: f64,
    pub sunrise_offset: i16,
    pub sunset_offset: i16,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            utc_offset: 0,
            pulse: false,
            cams: [WeeklyCam::default(); 3],
            recurrence: Recurrence::Yearly,
            start_date: [2000, 1, 1],
            end_date: [2099, 1, 2],
            latitude: 0.0,
            longitude: 0.0,
            sunrise_offset: 0,
            sunset_offset: 0,
        }
    }
}

impl Schedule {
    pub fn weekly(&self, now: i64, previous: i64) -> bool {
        let offset = i64::from(self.utc_offset) * 60;
        let now = now + offset;
        let previous = previous + offset;
        // Unix day zero was Thursday. Shift by three days to index from Monday.
        let week_time = (now + 3 * DAY).rem_euclid(WEEK);
        let mut latest = None;
        for cam in self.cams {
            let start = i64::from(cam.start_minute.min(1439)) * 60;
            let end = i64::from(cam.end_minute.min(1439)) * 60;
            for day in 0..7 {
                if cam.days & (1 << day) == 0 {
                    continue;
                }
                let on_elapsed = (week_time - i64::from(day) * DAY - start).rem_euclid(WEEK);
                if self.pulse && now - on_elapsed > previous {
                    return true;
                }
                for (elapsed, high) in [
                    (on_elapsed, true),
                    (
                        (week_time - i64::from(day) * DAY - end - i64::from(start > end) * DAY)
                            .rem_euclid(WEEK),
                        false,
                    ),
                ] {
                    if latest.is_none_or(|(last, _)| elapsed <= last) {
                        latest = Some((elapsed, high));
                    }
                }
            }
        }
        !self.pulse && latest.is_some_and(|(_, high)| high)
    }

    pub fn annual(&self, now: i64, previous: i64) -> bool {
        let offset = i64::from(self.utc_offset) * 60;
        let days = (now + offset).div_euclid(DAY);
        let date = date_at(days);
        let [year, month, day] = date;
        let [first, start_month, start_day] = self.start_date;
        let [last, end_month, end_day] = self.end_date;
        if self.pulse {
            let starts_today = match self.recurrence {
                Recurrence::Once => date == self.start_date,
                Recurrence::Monthly => (first..=last).contains(&year) && day == start_day,
                Recurrence::Yearly => {
                    (first..=last).contains(&year) && [month, day] == [start_month, start_day]
                }
            };
            return starts_today && days > (previous + offset).div_euclid(DAY);
        }
        match self.recurrence {
            Recurrence::Once => (self.start_date..self.end_date).contains(&date),
            Recurrence::Yearly => {
                (self.start_date..self.end_date).contains(&date)
                    && recurring([month, day], [start_month, start_day], [end_month, end_day])
            }
            Recurrence::Monthly => {
                (first..=last).contains(&year)
                    && (year != first || month != 1 || day >= start_day)
                    && (year != last || month != 12 || day < end_day)
                    && recurring(day, start_day, end_day)
            }
        }
    }

    pub fn daylight(&self, now: i64) -> bool {
        // Solar declination and equation of time from NOAA's fractional-year approximation.
        let solar_day = (now as f64 + self.longitude * 240.0).floor() as i64;
        let day = solar_day.div_euclid(DAY);
        let [year, month, date] = date_at(day);
        let ordinal = (1..month)
            .map(|month| f64::from(month_days(year, month)))
            .sum::<f64>()
            + f64::from(date)
            - 1.0;
        let g = std::f64::consts::TAU * ordinal / if leap_year(year) { 366.0 } else { 365.0 };
        let equation = 229.18
            * (0.000075 + 0.001868 * g.cos()
                - 0.032077 * g.sin()
                - 0.014615 * (2.0 * g).cos()
                - 0.040849 * (2.0 * g).sin());
        let declination = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin()
            - 0.006758 * (2.0 * g).cos()
            + 0.000907 * (2.0 * g).sin()
            - 0.002697 * (3.0 * g).cos()
            + 0.00148 * (3.0 * g).sin();
        let latitude = self.latitude.clamp(-89.999, 89.999).to_radians();
        let angle = (90.833_f64.to_radians().cos() - latitude.sin() * declination.sin())
            / (latitude.cos() * declination.cos());
        if angle >= 1.0 {
            return false;
        }
        if angle <= -1.0 {
            return true;
        }
        let half_day = 4.0 * angle.acos().to_degrees();
        let noon = 720.0 - equation;
        let sunrise = noon - half_day + f64::from(self.sunrise_offset);
        let sunset = noon + half_day + f64::from(self.sunset_offset);
        (sunrise..sunset).contains(&(solar_day.rem_euclid(DAY) as f64 / 60.0))
    }
}

fn recurring<T: Ord>(today: T, start: T, end: T) -> bool {
    if start <= end {
        (start..end).contains(&today)
    } else {
        today >= start || today < end
    }
}

fn leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

pub fn month_days(year: u16, month: u16) -> u16 {
    match month {
        2 => {
            if leap_year(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn date_at(mut day: i64) -> [u16; 3] {
    let mut year = 1970;
    day = day.max(0);
    while year < 9999 {
        let days = if leap_year(year) { 366 } else { 365 };
        if day < days {
            break;
        }
        day -= days;
        year += 1;
    }
    let mut month = 1;
    while month < 12 && day >= i64::from(month_days(year, month)) {
        day -= i64::from(month_days(year, month));
        month += 1;
    }
    [year, month, day.min(30) as u16 + 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weekly_window_continues_past_midnight_and_pulses_once_at_start() {
        let mut schedule = Schedule::default();
        schedule.cams[0] = WeeklyCam {
            days: 1 << 3,
            start_minute: 23 * 60,
            end_minute: 60,
        };
        let start = 23 * 3600;
        assert!(!schedule.weekly(start - 1, start - 2));
        assert!(schedule.weekly(DAY, DAY - 1));
        assert!(!schedule.weekly(DAY + 3600, DAY));
        schedule.pulse = true;
        assert!(schedule.weekly(start, start - 1));
        assert!(!schedule.weekly(start + 1, start));
    }

    #[test]
    fn civil_date_handles_a_leap_day() {
        assert_eq!(date_at(11_016), [2000, 2, 29]);
        assert_eq!(date_at(11_017), [2000, 3, 1]);
    }
}
