use crate::constants::{EXCEL_DATE_BASE, MAXIMUM_DATE_SERIAL_NUMBER};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub const DATE_OUT_OF_RANGE_MESSAGE: &str = "Out of range parameters for date";

pub fn calendar_offset(date1904: bool) -> i32 {
    if date1904 {
        1462
    } else {
        0
    }
}

/// Calendar fields retain Excel's day zero and fictional 1900 leap day.
/// A Gregorian NaiveDate cannot represent either of these worksheet values.
#[derive(Clone, Copy)]
pub struct CalendarComponents {
    year: i32,
    month: u32,
    day: u32,
    weekday: Weekday,
}
impl CalendarComponents {
    pub fn year(self) -> i32 {
        self.year
    }
    pub fn month(self) -> u32 {
        self.month
    }
    pub fn day(self) -> u32 {
        self.day
    }
    pub fn weekday(self) -> Weekday {
        self.weekday
    }
}
fn checked_serial(date1904: bool, serial: i64) -> Result<i64, String> {
    if !(0..=(MAXIMUM_DATE_SERIAL_NUMBER - calendar_offset(date1904)) as i64).contains(&serial) {
        return Err(DATE_OUT_OF_RANGE_MESSAGE.into());
    }
    Ok(serial + calendar_offset(date1904) as i64)
}
pub fn calendar_weekday(date1904: bool, serial: i64) -> Result<Weekday, String> {
    let serial = checked_serial(date1904, serial)?;
    // Excel retains Lotus's weekday sequence even before March 1900.
    Ok(match (serial + 5) % 7 {
        0 => Weekday::Mon,
        1 => Weekday::Tue,
        2 => Weekday::Wed,
        3 => Weekday::Thu,
        4 => Weekday::Fri,
        5 => Weekday::Sat,
        _ => Weekday::Sun,
    })
}
pub fn calendar_components(date1904: bool, serial: i64) -> Result<CalendarComponents, String> {
    let absolute = checked_serial(date1904, serial)?;
    let (year, month, day) = match absolute {
        0 => (1900, 1, 0),
        60 => (1900, 2, 29),
        _ => {
            let d = from_excel_date(absolute)?;
            (d.year(), d.month(), d.day())
        }
    };
    Ok(CalendarComponents {
        year,
        month,
        day,
        weekday: calendar_weekday(date1904, serial)?,
    })
}
/// Gregorian bridge for algorithms which require a real date. Serial 60 maps
/// to February 28; worksheet fields/month shifts/day stepping use the exact
/// component and serial helpers instead. Financial day-count conventions and
/// DATEDIF around the fictional day still require separate qualification.
pub fn calendar_date(date1904: bool, serial: i64) -> Result<NaiveDate, String> {
    from_excel_date(checked_serial(date1904, serial)?)
}
pub fn calendar_serial(date1904: bool, date: NaiveDate) -> Result<i32, String> {
    let serial = convert_to_serial_number(date) - calendar_offset(date1904);
    checked_serial(date1904, serial as i64)?;
    Ok(serial)
}
#[inline]
fn convert_to_serial_number(date: NaiveDate) -> i32 {
    let serial = date.num_days_from_ce() - EXCEL_DATE_BASE;
    if date.year() < 1900 || (date.year() == 1900 && date.month() < 3) {
        serial - 1
    } else {
        serial
    }
}
pub fn from_excel_date(serial: i64) -> Result<NaiveDate, String> {
    checked_serial(false, serial)?;
    let base = NaiveDate::from_ymd_opt(1899, 12, 31).ok_or(DATE_OUT_OF_RANGE_MESSAGE)?;
    Ok(base + Duration::days(serial - if serial >= 60 { 1 } else { 0 }))
}
pub fn date_to_serial_number(day: u32, month: u32, year: i32) -> Result<i32, String> {
    if (year, month, day) == (1900, 2, 29) {
        return Ok(60);
    }
    NaiveDate::from_ymd_opt(year, month, day)
        .map(convert_to_serial_number)
        .ok_or_else(|| DATE_OUT_OF_RANGE_MESSAGE.into())
}
fn normalized_month(year: i32, month: i64) -> Result<(i32, u32), String> {
    let total = year as i64 * 12 + month - 1;
    let year = i32::try_from(total.div_euclid(12)).map_err(|_| DATE_OUT_OF_RANGE_MESSAGE)?;
    Ok((year, (total.rem_euclid(12) + 1) as u32))
}
pub fn permissive_date_to_serial_number(day: i32, month: i32, year: i32) -> Result<i32, String> {
    let (year, month) = normalized_month(year, month as i64)?;
    // Add days in serial space so crossing February 1900 includes serial 60.
    let first = date_to_serial_number(1, month, year)? as i64;
    let serial = first + day as i64 - 1;
    checked_serial(false, serial)?;
    Ok(serial as i32)
}
fn month_length(year: i32, month: u32) -> u32 {
    // EDATE's clamp and EOMONTH's last day use Gregorian month lengths.
    // DATE/day stepping can still construct the fictional February 29, 1900.
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}
pub fn shift_calendar_months(
    date1904: bool,
    serial: i64,
    months: i32,
    end_of_month: bool,
) -> Result<i32, String> {
    let start = calendar_components(date1904, serial)?;
    let (year, month) = normalized_month(start.year(), start.month() as i64 + months as i64)?;
    if !(1900..=9999).contains(&year) {
        return Err(DATE_OUT_OF_RANGE_MESSAGE.into());
    }
    let last = month_length(year, month);
    let day = if end_of_month {
        last
    } else {
        start.day().min(last)
    };
    let result = if day == 0 {
        date_to_serial_number(1, month, year)? - 1
    } else {
        date_to_serial_number(day, month, year)?
    } - calendar_offset(date1904);
    checked_serial(date1904, result as i64)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissive_date_to_serial_number() {
        assert_eq!(
            permissive_date_to_serial_number(42, 42, 2002),
            date_to_serial_number(12, 7, 2005)
        );
        assert_eq!(
            permissive_date_to_serial_number(1, 42, 2002),
            date_to_serial_number(1, 6, 2005)
        );
        assert_eq!(
            permissive_date_to_serial_number(1, 15, 2000),
            date_to_serial_number(1, 3, 2001)
        );
        assert_eq!(
            permissive_date_to_serial_number(1, 49, 2000),
            date_to_serial_number(1, 1, 2004)
        );
        assert_eq!(
            permissive_date_to_serial_number(1, 49, 2000),
            date_to_serial_number(1, 1, 2004)
        );
        assert_eq!(
            permissive_date_to_serial_number(31, 49, 2000),
            date_to_serial_number(31, 1, 2004)
        );
        assert_eq!(
            permissive_date_to_serial_number(256, 49, 2000),
            date_to_serial_number(12, 9, 2004)
        );
        assert_eq!(
            permissive_date_to_serial_number(256, 1, 2004),
            date_to_serial_number(12, 9, 2004)
        );
    }

    #[test]
    fn test_max_and_min_dates() {
        assert_eq!(
            permissive_date_to_serial_number(31, 12, 9999),
            Ok(MAXIMUM_DATE_SERIAL_NUMBER),
        );
        assert_eq!(permissive_date_to_serial_number(31, 12, 1899), Ok(0),);
    }
}
