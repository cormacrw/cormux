//! ClickUp has no sprint endpoint. A sprint is a List inside a Sprint folder, with the
//! sprint's dates as the list's `start_date` and `due_date`.

use super::types::ClickupSprint;

/// The sprint running at `now_ms`: the list whose dates contain it. Between sprints, the
/// latest one that has started; before the first, the earliest upcoming one.
pub fn current_sprint(lists: &[ClickupSprint], now_ms: i64) -> Option<&ClickupSprint> {
    let running = lists
        .iter()
        .filter(|list| {
            list.start_ms.is_some_and(|start| start <= now_ms)
                && list.due_ms.is_some_and(|due| now_ms <= due)
        })
        .max_by_key(|list| list.start_ms);
    if running.is_some() {
        return running;
    }
    let started = lists
        .iter()
        .filter(|list| list.start_ms.is_some_and(|start| start <= now_ms))
        .max_by_key(|list| list.start_ms);
    if started.is_some() {
        return started;
    }
    lists
        .iter()
        .filter(|list| list.start_ms.is_some())
        .min_by_key(|list| list.start_ms)
        .or_else(|| lists.last())
}

/// ClickUp sends dates as millisecond strings, or null.
pub fn parse_ms(raw: Option<&str>) -> Option<i64> {
    raw.and_then(|value| value.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400_000;

    fn sprint(id: &str, start: Option<i64>, due: Option<i64>) -> ClickupSprint {
        ClickupSprint {
            id: id.into(),
            name: format!("Sprint {id}"),
            start_ms: start,
            due_ms: due,
        }
    }

    #[test]
    fn picks_the_sprint_whose_dates_contain_now() {
        let lists = [
            sprint("1", Some(0), Some(14 * DAY)),
            sprint("2", Some(14 * DAY), Some(28 * DAY)),
            sprint("3", Some(28 * DAY), Some(42 * DAY)),
        ];
        assert_eq!(current_sprint(&lists, 20 * DAY).unwrap().id, "2");
    }

    #[test]
    fn between_sprints_keeps_the_latest_started() {
        let lists = [
            sprint("1", Some(0), Some(10 * DAY)),
            sprint("2", Some(11 * DAY), Some(20 * DAY)),
            sprint("3", Some(30 * DAY), Some(40 * DAY)),
        ];
        assert_eq!(current_sprint(&lists, 25 * DAY).unwrap().id, "2");
    }

    #[test]
    fn before_the_first_sprint_takes_the_next_one() {
        let lists = [
            sprint("2", Some(20 * DAY), Some(30 * DAY)),
            sprint("1", Some(10 * DAY), Some(20 * DAY)),
        ];
        assert_eq!(current_sprint(&lists, 0).unwrap().id, "1");
    }

    #[test]
    fn undated_lists_fall_back_to_the_last() {
        let lists = [sprint("a", None, None), sprint("b", None, None)];
        assert_eq!(current_sprint(&lists, 0).unwrap().id, "b");
        assert!(current_sprint(&[], 0).is_none());
    }

    #[test]
    fn parses_millisecond_strings() {
        assert_eq!(parse_ms(Some("1727740800000")), Some(1_727_740_800_000));
        assert_eq!(parse_ms(Some("")), None);
        assert_eq!(parse_ms(None), None);
    }
}
