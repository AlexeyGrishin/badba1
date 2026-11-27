//! Records: groups of fields connected through links, with their fields in
//! depth-first order.

use std::collections::BTreeSet;

use crate::layout::{Field, Link, Record};

/// Groups `fields` into records using `links`, and orders each record's fields.
///
/// Record order starts at the field with the earliest position (smallest y,
/// then smallest x, of its frame's top-left corner) and walks depth-first,
/// visiting connected fields earliest position first and skipping fields
/// already visited.
///
/// This relies on field ids following that same position order, which is how
/// [`detect`](crate::detect) numbers fields: "earliest position" is then
/// "smallest id". `fields` must be numbered 1, 2, … in that order.
pub fn records(fields: &[Field], links: &[Link]) -> Vec<Record> {
    let n = fields.len();
    // neighbors[id] = connected field ids, ascending and without duplicates.
    let mut neighbors = vec![BTreeSet::new(); n + 1];
    for link in links {
        for &a in &link.fields {
            for &b in &link.fields {
                if a != b {
                    neighbors[a].insert(b);
                }
            }
        }
    }

    let mut visited = vec![false; n + 1];
    let mut records = Vec::new();
    for start in 1..=n {
        if visited[start] {
            continue;
        }
        let mut order = vec![start];
        visited[start] = true;
        // Explicit stack of (field, its remaining neighbors) instead of recursion.
        let mut stack = vec![neighbors[start].iter()];
        while let Some(next) = stack.last_mut().map(|it| it.next()) {
            match next {
                Some(&id) if !visited[id] => {
                    visited[id] = true;
                    order.push(id);
                    stack.push(neighbors[id].iter());
                }
                Some(_) => {}
                None => {
                    stack.pop();
                }
            }
        }
        records.push(Record {
            id: records.len() + 1,
            fields: order,
        });
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Borders, Rect};

    /// Builds fields from named positions, numbering them the way `detect`
    /// does (by y, then x), and returns each record as a list of names.
    fn ordered(
        positions: &[(&'static str, u32, u32)],
        links: &[&[&str]],
    ) -> Vec<Vec<&'static str>> {
        let mut named = positions.to_vec();
        named.sort_by_key(|&(_, x, y)| (y, x));
        let id_of = |name: &str| named.iter().position(|&(n, _, _)| n == name).unwrap() + 1;
        let fields: Vec<Field> = named
            .iter()
            .enumerate()
            .map(|(i, &(_, x, y))| Field {
                id: i + 1,
                outer: Rect::new(x, y, x + 9, y + 9),
                inner: Rect::new(x + 1, y + 1, x + 8, y + 8),
                borders: Borders {
                    top: 1,
                    right: 1,
                    bottom: 1,
                    left: 1,
                },
                kind: crate::types::TYPES[0],
                color: None,
                value: crate::types::Value::Int(0),
            })
            .collect();
        let links: Vec<Link> = links
            .iter()
            .enumerate()
            .map(|(i, names)| Link {
                id: i + 1,
                fields: names.iter().map(|n| id_of(n)).collect(),
            })
            .collect();
        let result = records(&fields, &links);
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.id, i + 1);
        }
        result
            .iter()
            .map(|r| r.fields.iter().map(|&id| named[id - 1].0).collect())
            .collect()
    }

    #[test]
    fn linked_fields_form_one_record() {
        let r = ordered(
            &[("A", 0, 0), ("B", 50, 50), ("C", 0, 100)],
            &[&["A", "B"], &["B", "C"]],
        );
        assert_eq!(r, vec![vec!["A", "B", "C"]]);
    }

    #[test]
    fn unconnected_groups_form_separate_records() {
        let r = ordered(
            &[("A", 0, 0), ("B", 50, 0), ("C", 0, 100), ("D", 50, 100)],
            &[&["A", "B"], &["C", "D"]],
        );
        assert_eq!(r, vec![vec!["A", "B"], vec!["C", "D"]]);
    }

    #[test]
    fn lone_field() {
        let r = ordered(&[("A", 0, 0), ("B", 50, 0), ("L", 0, 100)], &[&["A", "B"]]);
        assert_eq!(r, vec![vec!["A", "B"], vec!["L"]]);
    }

    #[test]
    fn link_joining_three_fields() {
        let r = ordered(
            &[("A", 0, 0), ("B", 50, 50), ("C", 100, 20)],
            &[&["A", "B", "C"]],
        );
        // C (y 20) comes before B (y 50); all three are connected pairwise.
        assert_eq!(r, vec![vec!["A", "C", "B"]]);
    }

    #[test]
    fn single_connection_continues_the_chain() {
        // The chain A–B–C: B is lower than C, but C is only reachable through B.
        let r = ordered(
            &[("A", 0, 0), ("B", 0, 200), ("C", 100, 100)],
            &[&["A", "B"], &["B", "C"]],
        );
        assert_eq!(r, vec![vec!["A", "B", "C"]]);
    }

    #[test]
    fn topmost_field_starts_the_record() {
        let r = ordered(
            &[("smiley", 35, 23), ("dots", 101, 22), ("green", 156, 22)],
            &[&["smiley", "dots"], &["dots", "green"]],
        );
        assert_eq!(r[0][0], "dots");
    }

    #[test]
    fn deeper_fields_come_before_later_siblings() {
        let r = ordered(
            &[("A", 0, 0), ("B", 100, 40), ("C", 0, 80), ("D", 100, 120)],
            &[&["A", "B"], &["A", "C"], &["B", "D"]],
        );
        assert_eq!(r, vec![vec!["A", "B", "D", "C"]]);
    }

    #[test]
    fn siblings_with_equal_y() {
        let r = ordered(
            &[("A", 50, 0), ("B", 100, 40), ("C", 0, 40)],
            &[&["A", "B"], &["A", "C"]],
        );
        assert_eq!(r, vec![vec!["A", "C", "B"]]);
    }

    #[test]
    fn loop_of_three_fields() {
        let r = ordered(
            &[("A", 0, 0), ("B", 100, 0), ("C", 50, 80)],
            &[&["A", "B"], &["B", "C"], &["C", "A"]],
        );
        assert_eq!(r, vec![vec!["A", "B", "C"]]);
    }

    #[test]
    fn parallel_links_do_not_repeat_fields() {
        let r = ordered(&[("A", 0, 0), ("B", 50, 0)], &[&["A", "B"], &["A", "B"]]);
        assert_eq!(r, vec![vec!["A", "B"]]);
    }

    #[test]
    fn two_records_numbered_by_first_field() {
        let r = ordered(
            &[
                ("red", 130, 69),
                ("frown", 72, 70),
                ("dots", 101, 22),
                ("green", 156, 22),
            ],
            &[&["red", "frown"], &["dots", "green"]],
        );
        assert_eq!(r, vec![vec!["dots", "green"], vec!["red", "frown"]]);
    }

    #[test]
    fn long_chain_does_not_overflow() {
        let n = 10_000;
        let fields: Vec<Field> = (0..n)
            .map(|i| Field {
                id: i + 1,
                outer: Rect::new(0, i as u32 * 3, 2, i as u32 * 3 + 2),
                inner: Rect::new(1, i as u32 * 3 + 1, 1, i as u32 * 3 + 1),
                borders: Borders {
                    top: 1,
                    right: 1,
                    bottom: 1,
                    left: 1,
                },
                kind: crate::types::TYPES[0],
                color: None,
                value: crate::types::Value::Int(0),
            })
            .collect();
        let links: Vec<Link> = (1..n)
            .map(|i| Link {
                id: i,
                fields: vec![i, i + 1],
            })
            .collect();
        let r = records(&fields, &links);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fields, (1..=n).collect::<Vec<_>>());
    }

    #[test]
    fn position_order_equals_id_order() {
        // Through `detect`: a field drawn at (100,22) gets a smaller id than one
        // at (35,23), and so starts their record.
        use crate::detect::tests::{blank, fill, frame};
        let mut img = blank(200, 80);
        frame(&mut img, Rect::new(100, 22, 139, 47), (5, 5, 5, 5));
        frame(&mut img, Rect::new(35, 23, 77, 52), (5, 5, 5, 5));
        fill(&mut img, Rect::new(78, 35, 99, 35), crate::FRAME_COLOR);
        let layout = crate::detect(&img);
        assert_eq!(layout.records.len(), 1);
        let first = layout.records[0].fields[0];
        assert_eq!(layout.fields[first - 1].outer.left, 100);
    }
}
