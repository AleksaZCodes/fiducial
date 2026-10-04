//! DC checks of a product's resistive set points, solved from its own netlist.
//!
//! A set point is a voltage a circuit's resistors decide: a USB-C sink's
//! CC pull-down read against a source's pull-up, a regulator's feedback
//! divider, a sense resistor's drop. The product declares what it expects
//! (`[[check]]` in `product.toml`: nets driven, nets measured, the window
//! each must land in); the resistors and what they connect are the ones the
//! declaration already has — the same nets that reach the board. So a check
//! fails when a resistor's value or a pin's net moves, not when someone
//! remembers to redo the arithmetic.
//!
//! The network is solved by nodal analysis: a textbook linear solve, kept
//! here as the wire router keeps A* (spec 2026-10-01). It covers what is
//! linear and static — resistors, and sources declared as voltages behind a
//! resistance. Anything that switches, saturates or moves in time (a
//! regulator's loop, a transistor, a capacitor's charge) is a simulator's job
//! (ngspice), not this one's, and a check cannot ask for it.

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

/// One declared check.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// What it shows, in words a reviewer reads: "CC1 reads as a sink to a
    /// default-USB source".
    pub name: String,
    /// Nets held at a voltage, each behind an optional resistance (a
    /// source's pull-up): `{ CC1 = { volts = 5.0, ohms = 56000 } }`. GND is
    /// always 0 V.
    pub drive: BTreeMap<String, Drive>,
    /// Where nets must land: `{ CC1 = [0.25, 0.61] }`, volts.
    pub expect: BTreeMap<String, [f64; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drive {
    pub volts: f64,
    #[serde(default)]
    pub ohms: f64,
}

/// A resistance from a part's `value`: `27R`, `4.7k`, `5.1k`, `1M`, `4k7`,
/// `100 Ω`, `0R` (a link). `None` for anything that is not one — a
/// capacitor's `10uF`, an inductor's `4.7uH`, an empty value.
pub fn ohms(value: &str) -> Option<f64> {
    let first = value.split_whitespace().next()?;
    let v = first.trim_end_matches(['Ω', 'Ω']).trim();
    let v = v.strip_suffix("ohm").unwrap_or(v);
    // `4k7`: the multiplier stands in for the decimal point.
    for (m, mul) in [('R', 1.0), ('r', 1.0), ('k', 1e3), ('K', 1e3), ('M', 1e6)] {
        if let Some((a, b)) = v.split_once(m) {
            if a.is_empty() {
                return None;
            }
            let whole: f64 = a.parse().ok()?;
            let frac = if b.is_empty() {
                0.0
            } else {
                if !b.chars().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                format!("0.{b}").parse::<f64>().ok()?
            };
            return Some((whole + frac) * mul);
        }
    }
    // A bare number with an ohm sign was handled above; a bare number alone
    // is ambiguous (a capacitor in pF?) and is not read as a resistance.
    if first.contains(['Ω', 'Ω']) {
        return v.parse().ok();
    }
    None
}

/// A resistor between two nets.
#[derive(Debug, Clone)]
pub struct Resistor {
    pub part: String,
    pub a: String,
    pub b: String,
    pub ohms: f64,
}

/// Each net's voltage with `check`'s drives applied to `rs`.
///
/// Nodal analysis: every net but GND and the stiffly driven ones is an
/// unknown; a drive behind a resistance is its Norton equivalent. A net the
/// check measures that nothing connects to anything driven is reported, not
/// guessed at.
pub fn solve(rs: &[Resistor], check: &Check) -> Result<BTreeMap<String, f64>> {
    let mut fixed: BTreeMap<String, f64> = BTreeMap::new();
    fixed.insert("GND".into(), 0.0);
    for (net, d) in &check.drive {
        if d.ohms <= 0.0 {
            fixed.insert(net.clone(), d.volts);
        }
    }
    let mut unknown: Vec<String> = Vec::new();
    let mut note = |n: &String| {
        if !fixed.contains_key(n) && !unknown.contains(n) {
            unknown.push(n.clone());
        }
    };
    for r in rs {
        note(&r.a);
        note(&r.b);
    }
    for (n, d) in &check.drive {
        if d.ohms > 0.0 {
            note(n);
        }
    }
    for n in check.expect.keys() {
        note(n);
    }
    let k = unknown.len();
    let at = |n: &str| unknown.iter().position(|u| u == n);
    // G·v = i, with a vanishing leak to ground so a floating net does not make
    // the system singular; such a net is found and reported below.
    let mut g = vec![vec![0.0_f64; k]; k];
    let mut i = vec![0.0_f64; k];
    let leak = 1e-12;
    for (d, row) in g.iter_mut().enumerate() {
        row[d] += leak;
    }
    let stamp =
        |a: &str, b: &str, cond: f64, g: &mut Vec<Vec<f64>>, i: &mut Vec<f64>| match (at(a), at(b))
        {
            (Some(p), Some(q)) => {
                g[p][p] += cond;
                g[q][q] += cond;
                g[p][q] -= cond;
                g[q][p] -= cond;
            }
            (Some(p), None) => {
                g[p][p] += cond;
                i[p] += cond * fixed[b];
            }
            (None, Some(q)) => {
                g[q][q] += cond;
                i[q] += cond * fixed[a];
            }
            (None, None) => {}
        };
    for r in rs {
        if r.ohms <= 0.0 {
            bail!(
                "part `{}` is a 0 Ω link between {} and {}: join the nets instead",
                r.part,
                r.a,
                r.b
            );
        }
        stamp(&r.a, &r.b, 1.0 / r.ohms, &mut g, &mut i);
    }
    for (n, d) in &check.drive {
        if d.ohms > 0.0 {
            if let Some(p) = at(n) {
                g[p][p] += 1.0 / d.ohms;
                i[p] += d.volts / d.ohms;
            }
        }
    }
    let v = gauss(g, i)
        .ok_or_else(|| anyhow!("check `{}`: the network has no solution", check.name))?;
    // Reachable from something driven, through resistors: otherwise floating.
    let mut reach: Vec<String> = fixed.keys().cloned().collect();
    reach.extend(check.drive.keys().cloned());
    loop {
        let before = reach.len();
        for r in rs {
            let (ha, hb) = (reach.contains(&r.a), reach.contains(&r.b));
            if ha && !hb {
                reach.push(r.b.clone());
            } else if hb && !ha {
                reach.push(r.a.clone());
            }
        }
        if reach.len() == before {
            break;
        }
    }
    let mut out = fixed;
    for (p, n) in unknown.iter().enumerate() {
        if reach.contains(n) {
            out.insert(n.clone(), v[p]);
        }
    }
    Ok(out)
}

fn gauss(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().partial_cmp(&a[y][c].abs()).unwrap())?;
        if a[p][c].abs() < 1e-300 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        let (top, rest) = a.split_at_mut(c + 1);
        let pivot = &top[c];
        for (k, row) in rest.iter_mut().enumerate() {
            let f = row[c] / pivot[c];
            if f != 0.0 {
                for (x, p) in row[c..].iter_mut().zip(&pivot[c..]) {
                    *x -= f * p;
                }
                b[c + 1 + k] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|k| a[r][k] * x[k]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    Some(x)
}

/// Run every check; each result as a line for `why`, and the failures.
pub fn run(rs: &[Resistor], checks: &[Check]) -> Result<(Vec<String>, Vec<String>)> {
    let mut why = Vec::new();
    let mut failed = Vec::new();
    for c in checks {
        if c.expect.is_empty() {
            bail!("check `{}` expects nothing: name a net and its window, e.g. expect = {{ CC1 = [0.25, 0.61] }}", c.name);
        }
        let v = solve(rs, c)?;
        for (net, [lo, hi]) in &c.expect {
            match v.get(net) {
                None => failed.push(format!(
                    "check `{}`: {net} connects to nothing the check drives — no resistor joins it",
                    c.name
                )),
                Some(x) if *x < lo - 1e-9 || *x > hi + 1e-9 => failed.push(format!(
                    "check `{}`: {net} is {:.3} V, outside {lo}–{hi} V",
                    c.name, x
                )),
                Some(x) => why.push(format!(
                    "check `{}`: {net} is {:.3} V, inside {lo}–{hi} V (solved from the declared resistors)",
                    c.name, x
                )),
            }
        }
    }
    Ok((why, failed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(drive: &[(&str, f64, f64)], expect: &[(&str, f64, f64)]) -> Check {
        Check {
            name: "t".into(),
            drive: drive
                .iter()
                .map(|(n, v, o)| {
                    (
                        n.to_string(),
                        Drive {
                            volts: *v,
                            ohms: *o,
                        },
                    )
                })
                .collect(),
            expect: expect
                .iter()
                .map(|(n, a, b)| (n.to_string(), [*a, *b]))
                .collect(),
        }
    }

    fn r(part: &str, a: &str, b: &str, ohms: f64) -> Resistor {
        Resistor {
            part: part.into(),
            a: a.into(),
            b: b.into(),
            ohms,
        }
    }

    #[test]
    fn values_read_as_resistances_and_others_do_not() {
        assert_eq!(ohms("27R"), Some(27.0));
        assert_eq!(ohms("5.1k"), Some(5100.0));
        assert_eq!(ohms("4k7"), Some(4700.0));
        assert_eq!(ohms("1M 1%"), Some(1e6));
        assert_eq!(ohms("100Ω"), Some(100.0));
        assert_eq!(ohms("10uF 10V X5R"), None);
        assert_eq!(ohms("33pF 50V C0G"), None);
        assert_eq!(ohms("4.7uH"), None);
        assert_eq!(ohms(""), None);
    }

    #[test]
    fn a_sink_pull_down_against_a_source_pull_up_is_a_divider() {
        // 5 V through 56 kΩ into 5.1 kΩ: 5 × 5.1 / 61.1 = 0.4173 V.
        let rs = [r("r-cc1", "CC1", "GND", 5100.0)];
        let v = solve(&rs, &check(&[("CC1", 5.0, 56000.0)], &[("CC1", 0.0, 1.0)])).unwrap();
        assert!((v["CC1"] - 0.41735).abs() < 1e-4, "{v:?}");
    }

    #[test]
    fn a_divider_between_driven_nets_and_a_floating_net() {
        // 3.3 V over 10 kΩ / 4.7 kΩ: 3.3 × 4.7 / 14.7 = 1.0551 V at FB.
        let rs = [
            r("top", "VOUT", "FB", 10000.0),
            r("bot", "FB", "GND", 4700.0),
            r("x", "A", "B", 1.0),
        ];
        let c = check(&[("VOUT", 3.3, 0.0)], &[("FB", 1.0, 1.1), ("A", 0.0, 1.0)]);
        let v = solve(&rs, &c).unwrap();
        assert!((v["FB"] - 1.05510).abs() < 1e-4, "{v:?}");
        let (why, failed) = run(&rs, &[c]).unwrap();
        assert_eq!(why.len(), 1);
        assert!(failed[0].contains("A connects to nothing"), "{failed:?}");
    }

    #[test]
    fn a_value_outside_its_window_fails_by_name() {
        let rs = [r("r-cc1", "CC1", "GND", 22000.0)];
        let (_, failed) = run(
            &rs,
            &[check(&[("CC1", 5.0, 56000.0)], &[("CC1", 0.25, 0.61)])],
        )
        .unwrap();
        assert!(
            failed[0].contains("CC1 is 1.410 V, outside 0.25–0.61 V"),
            "{failed:?}"
        );
    }
}
