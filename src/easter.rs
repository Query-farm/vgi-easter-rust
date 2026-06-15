// Copyright 2026 Query Farm LLC - https://query.farm

//! The Easter Sunday computation (Anonymous Gregorian *Computus*) and the
//! year -> `date32` (days-since-epoch) conversion used by the worker.

/// A simple `(year, month, day)` civil date — the result of the *Computus*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Return the Gregorian date of Easter Sunday for `year`.
///
/// Uses the Anonymous Gregorian algorithm (Meeus/Jones/Butcher *Computus*),
/// valid for any year in the Gregorian calendar (1583 onward). The integer
/// arithmetic mirrors the Python reference (`_easter_sunday`) exactly.
pub fn easter_sunday(year: i64) -> CivilDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let ell = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * ell) / 451;
    let month = (h + ell - 7 * m + 114) / 31;
    let day = ((h + ell - 7 * m + 114) % 31) + 1;
    CivilDate {
        year: year as i32,
        month: month as u32,
        day: day as u32,
    }
}

/// Convert a proleptic-Gregorian `(year, month, day)` to days since the Unix
/// epoch (1970-01-01), i.e. an Arrow `date32` value. Howard Hinnant's
/// `days_from_civil` algorithm.
pub fn days_from_civil(date: CivilDate) -> i32 {
    let y = if date.month <= 2 {
        date.year - 1
    } else {
        date.year
    } as i64;
    let m = date.month as i64;
    let d = date.day as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    (era * 146097 + doe - 719468) as i32
}

/// Easter Sunday for `year` as a `date32` value (days since the Unix epoch).
pub fn easter_date32(year: i64) -> i32 {
    days_from_civil(easter_sunday(year))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known Western (Gregorian) Easter Sunday dates.
    const KNOWN_EASTERS: &[(i64, i32, u32, u32)] = &[
        (2020, 2020, 4, 12),
        (2021, 2021, 4, 4),
        (2022, 2022, 4, 17),
        (2023, 2023, 4, 9),
        (2024, 2024, 3, 31),
        (2025, 2025, 4, 20),
        (2026, 2026, 4, 5),
        (2027, 2027, 3, 28),
        (2030, 2030, 4, 21),
        (1818, 1818, 3, 22), // earliest possible Easter (March 22)
        (1943, 1943, 4, 25), // latest possible Easter (April 25)
    ];

    #[test]
    fn test_easter_sunday() {
        for &(year, y, m, d) in KNOWN_EASTERS {
            assert_eq!(
                easter_sunday(year),
                CivilDate {
                    year: y,
                    month: m,
                    day: d
                },
                "easter_sunday({year})"
            );
        }
    }

    #[test]
    fn test_days_from_civil_epoch() {
        // Reference points for the days-since-epoch conversion.
        assert_eq!(
            days_from_civil(CivilDate {
                year: 1970,
                month: 1,
                day: 1
            }),
            0
        );
        assert_eq!(
            days_from_civil(CivilDate {
                year: 1970,
                month: 1,
                day: 2
            }),
            1
        );
        assert_eq!(
            days_from_civil(CivilDate {
                year: 1969,
                month: 12,
                day: 31
            }),
            -1
        );
        // 2025-04-20 is day 20198 since the epoch.
        assert_eq!(
            days_from_civil(CivilDate {
                year: 2025,
                month: 4,
                day: 20
            }),
            20198
        );
    }

    #[test]
    fn test_easter_date32_known() {
        // The full pipeline year -> date32 for a couple of known dates.
        assert_eq!(
            easter_date32(2025),
            days_from_civil(CivilDate {
                year: 2025,
                month: 4,
                day: 20
            })
        );
        assert_eq!(
            easter_date32(2024),
            days_from_civil(CivilDate {
                year: 2024,
                month: 3,
                day: 31
            })
        );
    }
}
