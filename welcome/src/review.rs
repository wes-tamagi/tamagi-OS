//! The review page: everything chosen, grouped by install method, with the
//! approximate total and what asks for the password.

use crate::method::Method;

/// One thing that will be installed.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Name shown (already translated).
    pub name: String,
    pub method: Method,
    pub size_mb: Option<f64>,
}

/// Drops repeats (the same thing chosen twice, say Firefox as the browser and
/// in the Office profile), keeping the first.
pub fn dedup(rows: Vec<Row>) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::with_capacity(rows.len());
    for row in rows {
        if !out.iter().any(|r| r.name == row.name && r.method == row.method) {
            out.push(row);
        }
    }
    out
}

/// The rows of each install method, in the order of the legend.
pub fn grouped(rows: &[Row]) -> Vec<(Method, Vec<&Row>)> {
    Method::ALL
        .iter()
        .filter_map(|&method| {
            let group: Vec<&Row> = rows.iter().filter(|r| r.method == method).collect();
            (!group.is_empty()).then_some((method, group))
        })
        .collect()
}

/// The sum of the known sizes.
pub fn total_mb(rows: &[Row]) -> f64 {
    rows.iter().filter_map(|r| r.size_mb).sum()
}

/// The names of what asks for the password.
pub fn password_names(rows: &[Row]) -> Vec<String> {
    rows.iter().filter(|r| r.method.asks_password()).map(|r| r.name.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, method: Method, size: Option<f64>) -> Row {
        Row { name: name.into(), method, size_mb: size }
    }

    fn sample() -> Vec<Row> {
        dedup(vec![
            row("Firefox", Method::Flatpak, Some(260.0)),
            row("Claude Code", Method::User, Some(220.0)),
            row("Steam + steam-devices", Method::Native, Some(900.0)),
            row("Firefox", Method::Flatpak, Some(260.0)),
            row("Fedora development box", Method::Box, Some(1536.0)),
            row("node@lts", Method::Box, None),
            row("Firefox", Method::Native, Some(260.0)),
        ])
    }

    #[test]
    fn repeats_are_dropped_but_not_other_methods() {
        let rows = sample();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows.iter().filter(|r| r.name == "Firefox").count(), 2);
    }

    #[test]
    fn groups() {
        let rows = sample();
        let groups: Vec<(Method, Vec<&str>)> = grouped(&rows).into_iter().map(|(m, g)| (m, g.iter().map(|r| r.name.as_str()).collect())).collect();
        assert_eq!(
            groups,
            [
                (Method::Flatpak, vec!["Firefox"]),
                (Method::Native, vec!["Steam + steam-devices", "Firefox"]),
                (Method::Box, vec!["Fedora development box", "node@lts"]),
                (Method::User, vec!["Claude Code"]),
            ]
        );
    }

    #[test]
    fn totals() {
        let rows = sample();
        assert_eq!(total_mb(&rows), 260.0 + 220.0 + 900.0 + 1536.0 + 260.0);
        assert_eq!(password_names(&rows), ["Steam + steam-devices", "Firefox"]);
        assert_eq!(total_mb(&[]), 0.0);
        assert!(password_names(&[]).is_empty());
    }
}
