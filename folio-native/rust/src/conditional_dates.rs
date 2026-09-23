//! Excel's ten date-occurring predicates; boundaries use the calculation clock.
pub const PERIODS: &[&str] = &[
    "yesterday",
    "today",
    "tomorrow",
    "last7Days",
    "lastWeek",
    "thisWeek",
    "nextWeek",
    "lastMonth",
    "thisMonth",
    "nextMonth",
];
pub fn formula(period: &str, cell: &str) -> Result<String, String> {
    // Half-open serial intervals include all times on the final day and cross
    // year/month boundaries without comparing month numbers in isolation.
    let (start, end) = match period {
        "yesterday" => ("TODAY()-1".to_owned(), "TODAY()".to_owned()),
        "today" => ("TODAY()".to_owned(), "TODAY()+1".to_owned()),
        "tomorrow" => ("TODAY()+1".to_owned(), "TODAY()+2".to_owned()),
        "last7Days" => ("TODAY()-6".to_owned(), "TODAY()+1".to_owned()),
        "lastWeek" => (
            "TODAY()-WEEKDAY(TODAY())-6".to_owned(),
            "TODAY()-WEEKDAY(TODAY())+1".to_owned(),
        ),
        "thisWeek" => (
            "TODAY()-WEEKDAY(TODAY())+1".to_owned(),
            "TODAY()-WEEKDAY(TODAY())+8".to_owned(),
        ),
        "nextWeek" => (
            "TODAY()-WEEKDAY(TODAY())+8".to_owned(),
            "TODAY()-WEEKDAY(TODAY())+15".to_owned(),
        ),
        "lastMonth" => (
            "DATE(YEAR(TODAY()),MONTH(TODAY())-1,1)".to_owned(),
            "DATE(YEAR(TODAY()),MONTH(TODAY()),1)".to_owned(),
        ),
        "thisMonth" => (
            "DATE(YEAR(TODAY()),MONTH(TODAY()),1)".to_owned(),
            "DATE(YEAR(TODAY()),MONTH(TODAY())+1,1)".to_owned(),
        ),
        "nextMonth" => (
            "DATE(YEAR(TODAY()),MONTH(TODAY())+1,1)".to_owned(),
            "DATE(YEAR(TODAY()),MONTH(TODAY())+2,1)".to_owned(),
        ),
        _ => return Err("Choose a supported date period".into()),
    };
    Ok(format!(
        "AND(ISNUMBER({cell}),{cell}>=({start}),{cell}<({end}))"
    ))
}
