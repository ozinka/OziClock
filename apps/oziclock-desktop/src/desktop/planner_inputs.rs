use chrono::{NaiveDateTime, NaiveTime, Timelike};
use oziclock_storage::{AlertRule, AppSettings, EventRecurrence};

pub(super) fn duration_parts(total_seconds: u64) -> (u16, u8, u8, u8) {
    let days = (total_seconds / 86_400).min(u64::from(u16::MAX)) as u16;
    let hours = ((total_seconds % 86_400) / 3_600) as u8;
    let minutes = ((total_seconds % 3_600) / 60) as u8;
    let seconds = (total_seconds % 60) as u8;
    (days, hours, minutes, seconds)
}

fn timer_part_limit(part: i32) -> Option<u16> {
    match part {
        0 => Some(999),
        1 => Some(23),
        2 | 3 => Some(59),
        _ => None,
    }
}

pub(super) fn normalize_alarm_time(text: &str) -> Option<String> {
    let (hours, minutes) = text.split_once(':')?;
    let hours = parse_timer_part(hours, 1)?;
    let minutes = parse_timer_part(minutes, 2)?;
    Some(
        NaiveTime::from_hms_opt(hours.into(), minutes.into(), 0)?
            .format("%H:%M")
            .to_string(),
    )
}

pub(super) fn default_alarm_time(now: NaiveDateTime) -> String {
    let current_seconds = now.time().num_seconds_from_midnight();
    let rounded_five_minutes = current_seconds.div_ceil(300) * 300;
    let target_minutes = (rounded_five_minutes / 60 + 5) % (24 * 60);
    format!("{:02}:{:02}", target_minutes / 60, target_minutes % 60)
}

pub(super) fn parse_timer_part(text: &str, part: i32) -> Option<u16> {
    let maximum = timer_part_limit(part)?;
    let value = text.trim().parse::<u16>().ok()?;
    (value <= maximum).then_some(value)
}

pub(super) fn mask_timer_part(text: &str, part: i32) -> Option<String> {
    let maximum = timer_part_limit(part)?;
    let maximum_digits = if part == 0 { 3 } else { 2 };
    let digits = text
        .chars()
        .filter(char::is_ascii_digit)
        .take(maximum_digits)
        .collect::<String>();
    if digits.is_empty() {
        return Some(String::new());
    }
    let value = digits.parse::<u16>().ok()?;
    if value <= maximum {
        Some(digits)
    } else {
        Some(maximum.to_string())
    }
}

pub(super) fn mask_clock_time(text: &str) -> String {
    let digits = text
        .chars()
        .filter(char::is_ascii_digit)
        .take(4)
        .collect::<String>();
    let hour_digits = digits.len().min(2);
    let mut result = digits[..hour_digits].to_string();
    if hour_digits == 2 {
        let hours = result.parse::<u8>().unwrap_or_default().min(23);
        result = format!("{hours:02}:");
    }
    if digits.len() > 2 {
        let minute_digits = &digits[2..];
        let minutes = minute_digits.parse::<u8>().unwrap_or_default().min(59);
        if minute_digits.len() == 1 {
            result.push_str(&minutes.to_string());
        } else {
            result.push_str(&format!("{minutes:02}"));
        }
    }
    result
}

pub(super) fn parse_event_recurrence(value: &str) -> Option<EventRecurrence> {
    match value {
        "Does not repeat" => Some(EventRecurrence::None),
        "Daily" => Some(EventRecurrence::Daily),
        "Weekly" => Some(EventRecurrence::Weekly),
        "Monthly" => Some(EventRecurrence::Monthly),
        "Yearly" => Some(EventRecurrence::Yearly),
        _ => None,
    }
}

pub(super) fn event_recurrence_label(recurrence: EventRecurrence) -> &'static str {
    match recurrence {
        EventRecurrence::None => "Does not repeat",
        EventRecurrence::Daily => "Daily",
        EventRecurrence::Weekly => "Weekly",
        EventRecurrence::Monthly => "Monthly",
        EventRecurrence::Yearly => "Yearly",
    }
}

fn parse_event_alert(value: &str) -> Option<Option<i32>> {
    match value {
        "No alert" => Some(None),
        "At start" => Some(Some(0)),
        "5 minutes before" => Some(Some(5)),
        "15 minutes before" => Some(Some(15)),
        "30 minutes before" => Some(Some(30)),
        "1 hour before" => Some(Some(60)),
        _ => None,
    }
}

pub(super) fn parse_alert_with_custom(
    value: &str,
    custom: &str,
    zero_label: &str,
) -> Option<Option<i32>> {
    if value == zero_label {
        return Some(Some(0));
    }
    if value == "Custom" {
        let minutes = custom.trim().parse::<i32>().ok()?;
        return (1..=10_080).contains(&minutes).then_some(Some(minutes));
    }
    parse_event_alert(value)
}

pub(super) fn event_alert_label(alert: Option<&AlertRule>) -> &'static str {
    match alert.map(|alert| alert.offset_minutes) {
        None => "No alert",
        Some(0) => "At start",
        Some(5) => "5 minutes before",
        Some(15) => "15 minutes before",
        Some(30) => "30 minutes before",
        Some(60) => "1 hour before",
        Some(_) => "Custom",
    }
}

pub(super) fn task_alert_label(offset_minutes: i32) -> &'static str {
    match offset_minutes {
        0 => "At due time",
        5 => "5 minutes before",
        15 => "15 minutes before",
        30 => "30 minutes before",
        60 => "1 hour before",
        _ => "Custom",
    }
}

pub(super) fn adjust_timer_part(text: &str, part: i32, direction: i32) -> Option<u16> {
    let value = if text.trim().is_empty() {
        0
    } else {
        parse_timer_part(text, part)?
    };
    Some(
        (i32::from(value) + direction.signum()).clamp(0, i32::from(timer_part_limit(part)?)) as u16,
    )
}

pub(super) fn store_timer_part(settings: &mut AppSettings, part: i32, value: u16) {
    match part {
        0 => settings.timer_draft_days = value,
        1 => settings.timer_draft_hours = value as u8,
        2 => settings.timer_draft_minutes = value as u8,
        3 => settings.timer_draft_seconds = value as u8,
        _ => {}
    }
}

pub(super) fn parse_timer_duration(
    days: &str,
    hours: &str,
    minutes: &str,
    seconds: &str,
) -> Option<(u16, u8, u8, u8)> {
    let days = parse_timer_part(days, 0)?;
    let hours = parse_timer_part(hours, 1)? as u8;
    let minutes = parse_timer_part(minutes, 2)? as u8;
    let seconds = parse_timer_part(seconds, 3)? as u8;
    Some((days, hours, minutes, seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alm_14_default_rounds_up_to_five_minutes_then_adds_five() {
        let at = |value| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!(default_alarm_time(at("2026-09-05 10:02:00")), "10:10");
        assert_eq!(default_alarm_time(at("2026-09-05 10:05:00")), "10:10");
        assert_eq!(default_alarm_time(at("2026-09-05 10:05:01")), "10:15");
        assert_eq!(default_alarm_time(at("2026-09-05 23:58:00")), "00:05");
    }

    #[test]
    fn alm_01_accepts_single_digits_and_normalizes_storage() {
        assert_eq!(normalize_alarm_time("6:0").as_deref(), Some("06:00"));
        assert_eq!(normalize_alarm_time("0:0").as_deref(), Some("00:00"));
        assert_eq!(normalize_alarm_time("23:59").as_deref(), Some("23:59"));
        for invalid in ["24:00", "12:60", "-1:30", ":30", "7:", "7:00:00"] {
            assert_eq!(normalize_alarm_time(invalid), None);
        }
    }

    #[test]
    fn tmr_01_tmr_02_input_masks_remove_invalid_characters_and_enforce_ranges() {
        assert_eq!(mask_timer_part("0a7", 1).as_deref(), Some("07"));
        assert_eq!(mask_timer_part("99", 2).as_deref(), Some("59"));
        assert_eq!(mask_timer_part("1234", 0).as_deref(), Some("123"));
        assert_eq!(mask_clock_time("1"), "1");
        assert_eq!(mask_clock_time("12"), "12:");
        assert_eq!(mask_clock_time("12x34"), "12:34");
        assert_eq!(mask_clock_time("2968"), "23:59");
    }

    #[test]
    fn tmr_01_tmr_02_adjustment_uses_current_text_and_stops_at_limits() {
        assert_eq!(adjust_timer_part("17", 2, 1), Some(18));
        assert_eq!(adjust_timer_part("0", 0, -1), Some(0));
        assert_eq!(adjust_timer_part("23", 1, 1), Some(23));
        assert_eq!(adjust_timer_part("59", 3, 1), Some(59));
        assert_eq!(adjust_timer_part("", 2, 1), Some(1));
        assert_eq!(adjust_timer_part("invalid", 2, 1), None);
    }

    #[test]
    fn tmr_02_duration_rejects_invalid_parts_without_silent_truncation() {
        assert_eq!(parse_timer_duration("0", "0", "5", "0"), Some((0, 0, 5, 0)));
        assert_eq!(
            parse_timer_duration("999", "23", "59", "59"),
            Some((999, 23, 59, 59))
        );
        for parts in [
            ["1000", "0", "0", "0"],
            ["0", "24", "0", "0"],
            ["0", "0", "60", "0"],
            ["0", "0", "0", "-1"],
            ["", "0", "5", "0"],
        ] {
            assert_eq!(
                parse_timer_duration(parts[0], parts[1], parts[2], parts[3]),
                None
            );
        }
    }

    #[test]
    fn evt_08_recurrence_labels_round_trip() {
        for (label, recurrence) in [
            ("Does not repeat", EventRecurrence::None),
            ("Daily", EventRecurrence::Daily),
            ("Weekly", EventRecurrence::Weekly),
            ("Monthly", EventRecurrence::Monthly),
            ("Yearly", EventRecurrence::Yearly),
        ] {
            assert_eq!(parse_event_recurrence(label), Some(recurrence));
            assert_eq!(event_recurrence_label(recurrence), label);
        }
        assert_eq!(parse_event_recurrence("Unknown"), None);
    }

    #[test]
    fn evt_09_evt_12_tsk_12_alert_mapping_accepts_presets_and_custom_limits() {
        assert_eq!(
            parse_alert_with_custom("No alert", "", "At start"),
            Some(None)
        );
        assert_eq!(
            parse_alert_with_custom("At start", "", "At start"),
            Some(Some(0))
        );
        assert_eq!(
            parse_alert_with_custom("Custom", "10080", "At start"),
            Some(Some(10_080))
        );
        assert_eq!(parse_alert_with_custom("Custom", "0", "At start"), None);
        assert_eq!(parse_alert_with_custom("Custom", "10081", "At start"), None);
        assert_eq!(task_alert_label(0), "At due time");
        assert_eq!(task_alert_label(61), "Custom");
    }
}
