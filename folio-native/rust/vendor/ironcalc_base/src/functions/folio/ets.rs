//! The FORECAST.ETS family: additive exponential triple smoothing (AAA).
//!
//! Excel doesn't publish how it fits the smoothing parameters or detects
//! seasonality, so these follow the textbook method: the timeline is sorted,
//! duplicates are aggregated and gaps filled, the seasonality is the
//! strongest autocorrelation peak, and alpha, beta and gamma minimize the
//! one-step squared error. Results are close to Excel's, not identical.
use crate::{
    calc_result::CalcResult,
    expressions::{
        parser::{ArrayNode, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    model::Model,
};
use statrs::distribution::{ContinuousCDF, Normal};

/// The longest season Excel accepts: 8,760 (hours in a year).
const MAX_SEASON: usize = 8760;

/// A regular series built from a timeline and its values.
#[derive(Debug, Clone)]
pub(crate) struct Series {
    pub values: Vec<f64>,
    pub start: f64,
    pub step: f64,
}

/// Sorts the points, aggregates duplicates (1 AVERAGE, 2 COUNT, 3 COUNTA,
/// 4 MAX, 5 MEDIAN, 6 MIN, 7 SUM) and fills missing steps, by linear
/// interpolation (`complete`) or with zeros.
pub(crate) fn series(
    timeline: &[f64],
    values: &[Option<f64>],
    complete: bool,
    aggregation: i32,
) -> Result<Series, Error> {
    if timeline.len() != values.len() || timeline.len() < 3 || !(1..=7).contains(&aggregation) {
        return Err(Error::NUM);
    }
    let mut points: Vec<(f64, Option<f64>)> = timeline
        .iter()
        .copied()
        .zip(values.iter().copied())
        .collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, Vec<f64>, usize)> = vec![];
    for (x, y) in points {
        match merged.last_mut() {
            Some(last) if last.0 == x => {
                last.2 += 1;
                last.1.extend(y);
            }
            _ => merged.push((x, y.into_iter().collect(), 1)),
        }
    }
    let aggregate = |ys: &[f64], count: usize| -> Option<f64> {
        let mut v = ys.to_vec();
        v.sort_by(|a, b| a.total_cmp(b));
        Some(match aggregation {
            2 => ys.len() as f64,
            3 => count as f64,
            _ if ys.is_empty() => return None,
            1 => ys.iter().sum::<f64>() / ys.len() as f64,
            4 => v[v.len() - 1],
            5 => {
                let m = v.len() / 2;
                if v.len() % 2 == 1 {
                    v[m]
                } else {
                    (v[m - 1] + v[m]) / 2.0
                }
            }
            6 => v[0],
            _ => ys.iter().sum(),
        })
    };
    let xs: Vec<f64> = merged.iter().map(|m| m.0).collect();
    let step = xs
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min);
    if !(step > 0.0 && step.is_finite()) {
        return Err(Error::NUM);
    }
    // Every gap must be a whole number of steps.
    let mut slots: Vec<Option<f64>> = vec![];
    for (i, (x, ys, count)) in merged.iter().enumerate() {
        let index = (x - xs[0]) / step;
        if (index - index.round()).abs() > 1e-6 {
            return Err(Error::NUM);
        }
        let index = index.round() as usize;
        if index > 10 * timeline.len() + 10 {
            return Err(Error::NUM);
        }
        slots.resize(index, None);
        slots.push(aggregate(ys, *count));
        let _ = i;
    }
    // Excel accepts up to 30% missing points.
    let missing = slots.iter().filter(|s| s.is_none()).count();
    if missing as f64 > 0.3 * slots.len() as f64 {
        return Err(Error::NUM);
    }
    let known: Vec<(usize, f64)> = slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.map(|v| (i, v)))
        .collect();
    if known.len() < 2 {
        return Err(Error::NUM);
    }
    let filled = (0..slots.len())
        .map(|i| match slots[i] {
            Some(v) => v,
            None if !complete => 0.0,
            None => {
                let before = known.iter().rev().find(|(j, _)| *j < i);
                let after = known.iter().find(|(j, _)| *j > i);
                match (before, after) {
                    (Some((a, va)), Some((b, vb))) => {
                        va + (vb - va) * (i - a) as f64 / (b - a) as f64
                    }
                    (Some((_, v)), None) | (None, Some((_, v))) => *v,
                    (None, None) => 0.0,
                }
            }
        })
        .collect();
    Ok(Series {
        values: filled,
        start: xs[0],
        step,
    })
}

/// The season length with the strongest autocorrelation peak of the
/// detrended series, or 0 when there is none.
pub(crate) fn detect_season(y: &[f64]) -> usize {
    let n = y.len();
    if n < 6 {
        return 0;
    }
    // Remove the linear trend.
    let xm = (n - 1) as f64 / 2.0;
    let ym = y.iter().sum::<f64>() / n as f64;
    let sxy: f64 = y
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64 - xm) * (v - ym))
        .sum();
    let sxx: f64 = (0..n).map(|i| (i as f64 - xm).powi(2)).sum();
    let slope = sxy / sxx;
    let r: Vec<f64> = y
        .iter()
        .enumerate()
        .map(|(i, v)| v - ym - slope * (i as f64 - xm))
        .collect();
    let var: f64 = r.iter().map(|v| v * v).sum();
    if var <= 1e-12 {
        return 0;
    }
    let max_lag = (n / 2).min(MAX_SEASON);
    let acf: Vec<f64> = (0..=max_lag)
        .map(|lag| (lag..n).map(|i| r[i] * r[i - lag]).sum::<f64>() / var)
        .collect();
    let mut best = (0, 0.0);
    for lag in 2..max_lag {
        let peak = acf[lag] > acf[lag - 1] && acf[lag] >= acf[lag + 1];
        if peak && acf[lag] > best.1 {
            best = (lag, acf[lag]);
        }
    }
    // A weak peak is noise, not a season.
    if best.1 >= 0.3 {
        best.0
    } else {
        0
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Fit {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub season: usize,
    level: f64,
    trend: f64,
    seasonals: Vec<f64>,
    /// One-step-ahead predictions, aligned with the series.
    fitted: Vec<f64>,
    pub sse: f64,
}

fn run(y: &[f64], season: usize, alpha: f64, beta: f64, gamma: f64) -> Fit {
    let n = y.len();
    let s = season;
    let (mut level, mut trend, mut seasonals) = if s >= 2 {
        let first = y[..s].iter().sum::<f64>() / s as f64;
        let trend = if n >= 2 * s {
            (y[s..2 * s].iter().sum::<f64>() / s as f64 - first) / s as f64
        } else {
            (y[s - 1] - y[0]) / (s - 1) as f64
        };
        (
            first,
            trend,
            y[..s].iter().map(|v| v - first).collect::<Vec<_>>(),
        )
    } else {
        (y[0], y[1] - y[0], vec![0.0])
    };
    let mut fitted = vec![f64::NAN; n];
    let mut sse = 0.0;
    let begin = if s >= 2 { s } else { 1 };
    // Level and trend describe the point before `begin`.
    if s >= 2 {
        level += trend * (s as f64 - 1.0) / 2.0;
    } else {
        level = y[0];
    }
    for (t, value) in y.iter().enumerate().skip(begin) {
        let season_index = if s >= 2 { t % s } else { 0 };
        let seasonal = seasonals[season_index];
        let prediction = level + trend + seasonal;
        fitted[t] = prediction;
        sse += (value - prediction).powi(2);
        let previous = level;
        level = alpha * (value - seasonal) + (1.0 - alpha) * (level + trend);
        trend = beta * (level - previous) + (1.0 - beta) * trend;
        if s >= 2 {
            seasonals[season_index] = gamma * (value - level) + (1.0 - gamma) * seasonal;
        }
    }
    Fit {
        alpha,
        beta,
        gamma: if s >= 2 { gamma } else { 0.0 },
        season: s,
        level,
        trend,
        seasonals,
        fitted,
        sse,
    }
}

/// Fits alpha, beta and gamma by minimizing the one-step squared error:
/// a coarse grid, then a shrinking coordinate search.
pub(crate) fn fit(y: &[f64], season: usize) -> Fit {
    let season = if season >= 2 && y.len() >= 2 * season {
        season
    } else {
        0
    };
    let seasonal = season >= 2;
    let mut best = run(y, season, 0.5, 0.1, 0.1);
    let grid = [0.05, 0.2, 0.4, 0.6, 0.8, 0.95];
    for &a in &grid {
        for &b in &grid[..4] {
            for &g in if seasonal { &grid[..] } else { &grid[..1] } {
                let f = run(y, season, a, b, g);
                if f.sse < best.sse {
                    best = f;
                }
            }
        }
    }
    let mut step = 0.1;
    while step > 1e-4 {
        let mut improved = false;
        for which in 0..if seasonal { 3 } else { 2 } {
            for delta in [-step, step] {
                let mut p = [best.alpha, best.beta, best.gamma];
                p[which] = (p[which] + delta).clamp(0.001, 1.0);
                let f = run(y, season, p[0], p[1], p[2]);
                if f.sse < best.sse - 1e-12 {
                    best = f;
                    improved = true;
                }
            }
        }
        if !improved {
            step /= 2.0;
        }
    }
    best
}

impl Fit {
    /// The forecast `h` steps after the last point (fractional steps are
    /// interpolated).
    pub(crate) fn forecast(&self, n: usize, h: f64) -> f64 {
        let at = |k: usize| -> f64 {
            let seasonal = if self.season >= 2 {
                self.seasonals[(n - 1 + k) % self.season]
            } else {
                0.0
            };
            self.level + k as f64 * self.trend + seasonal
        };
        if h <= 0.0 {
            return at(0);
        }
        let low = h.floor() as usize;
        let frac = h - low as f64;
        if frac == 0.0 {
            at(low)
        } else {
            at(low) + frac * (at(low + 1) - at(low))
        }
    }

    fn residuals<'a>(&'a self, y: &'a [f64]) -> impl Iterator<Item = (f64, f64)> + 'a {
        y.iter()
            .zip(&self.fitted)
            .filter(|(_, f)| !f.is_nan())
            .map(|(a, f)| (*a, *f))
    }

    pub(crate) fn mse(&self, y: &[f64]) -> f64 {
        let r: Vec<f64> = self.residuals(y).map(|(a, f)| (a - f).powi(2)).collect();
        r.iter().sum::<f64>() / r.len().max(1) as f64
    }

    /// The half-width of the prediction interval `h` steps ahead.
    pub(crate) fn interval(&self, y: &[f64], h: f64, confidence: f64) -> f64 {
        let steps = h.ceil().max(1.0) as usize;
        let mut variance = 1.0;
        for j in 1..steps {
            let seasonal = if self.season >= 2 && j % self.season == 0 {
                self.gamma
            } else {
                0.0
            };
            variance += (self.alpha * (1.0 + j as f64 * self.beta) + seasonal).powi(2);
        }
        let z = Normal::new(0.0, 1.0)
            .map(|d| d.inverse_cdf(0.5 + confidence / 2.0))
            .unwrap_or(1.96);
        z * (self.mse(y) * variance).sqrt()
    }

    /// FORECAST.ETS.STAT: 1 alpha, 2 beta, 3 gamma, 4 MASE, 5 SMAPE, 6 MAE,
    /// 7 RMSE.
    pub(crate) fn statistic(&self, y: &[f64], kind: i32) -> Option<f64> {
        let r: Vec<(f64, f64)> = self.residuals(y).collect();
        let n = r.len().max(1) as f64;
        let mae = r.iter().map(|(a, f)| (a - f).abs()).sum::<f64>() / n;
        Some(match kind {
            1 => self.alpha,
            2 => self.beta,
            3 => self.gamma,
            4 => {
                let naive =
                    y.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (y.len() - 1) as f64;
                mae / naive
            }
            5 => {
                r.iter()
                    .map(|(a, f)| {
                        let d = (a.abs() + f.abs()) / 2.0;
                        if d == 0.0 {
                            0.0
                        } else {
                            (a - f).abs() / d
                        }
                    })
                    .sum::<f64>()
                    / n
            }
            6 => mae,
            7 => self.mse(y).sqrt(),
            _ => return None,
        })
    }
}

/// The arguments after the timeline, where each function puts them.
struct EtsArgs {
    seasonality: Option<usize>,
    complete: bool,
    aggregation: i32,
}

impl Model<'_> {
    fn ets_numbers(
        &mut self,
        node: &Node,
        cell: CellReferenceIndex,
        blanks: bool,
    ) -> Result<Vec<Option<f64>>, CalcResult> {
        let grid = self.eval_to_array(node, cell)?;
        let mut out = vec![];
        for v in grid.into_iter().flatten() {
            out.push(match v {
                ArrayNode::Number(n) => Some(n),
                ArrayNode::Empty if blanks => None,
                ArrayNode::Error(e) => return Err(CalcResult::new_error(e, cell, String::new())),
                _ => {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Expected numbers".to_string(),
                    ))
                }
            });
        }
        Ok(out)
    }

    fn ets_options(
        &mut self,
        args: &[Node],
        season_at: Option<usize>,
        complete_at: usize,
        cell: CellReferenceIndex,
    ) -> Result<EtsArgs, CalcResult> {
        let number = |model: &mut Self, i: usize, default: f64| -> Result<f64, CalcResult> {
            match args.get(i) {
                None | Some(Node::EmptyArgKind) => Ok(default),
                Some(a) => model.get_number(a, cell),
            }
        };
        let seasonality = match season_at {
            Some(i) => {
                let s = number(self, i, 1.0)?.trunc();
                if !(0.0..=MAX_SEASON as f64).contains(&s) {
                    return Err(CalcResult::new_error(
                        Error::NUM,
                        cell,
                        "Invalid seasonality".to_string(),
                    ));
                }
                // 1 detects the season; 0 has none.
                if s == 1.0 {
                    None
                } else {
                    Some(s as usize)
                }
            }
            None => None,
        };
        let complete = number(self, complete_at, 1.0)?.trunc();
        let aggregation = number(self, complete_at + 1, 1.0)?.trunc();
        if !(complete == 0.0 || complete == 1.0) || !(1.0..=7.0).contains(&aggregation) {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "Invalid option".to_string(),
            ));
        }
        Ok(EtsArgs {
            seasonality,
            complete: complete == 1.0,
            aggregation: aggregation as i32,
        })
    }

    /// The series and its fit from `values` and `timeline`.
    fn ets_fit(
        &mut self,
        values: &Node,
        timeline: &Node,
        options: &EtsArgs,
        cell: CellReferenceIndex,
    ) -> Result<(Series, Fit), CalcResult> {
        let values = self.ets_numbers(values, cell, true)?;
        let timeline: Vec<f64> = self
            .ets_numbers(timeline, cell, false)?
            .into_iter()
            .map(|v| v.unwrap_or(0.0))
            .collect();
        let series = series(&timeline, &values, options.complete, options.aggregation)
            .map_err(|e| CalcResult::new_error(e, cell, "Invalid timeline".to_string()))?;
        let season = options
            .seasonality
            .unwrap_or_else(|| detect_season(&series.values));
        let fit = fit(&series.values, season);
        Ok((series, fit))
    }

    fn ets_steps(
        &mut self,
        target: &Node,
        series: &Series,
        cell: CellReferenceIndex,
    ) -> Result<f64, CalcResult> {
        let target = self.get_number(target, cell)?;
        let last = series.start + series.step * (series.values.len() - 1) as f64;
        if target < last {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "The target date is before the end of the timeline".to_string(),
            ));
        }
        Ok((target - last) / series.step)
    }

    pub(crate) fn fn_forecast_ets(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if !(3..=6).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let run = |m: &mut Self| -> Result<f64, CalcResult> {
            let options = m.ets_options(args, Some(3), 4, cell)?;
            let (series, fit) = m.ets_fit(&args[1], &args[2], &options, cell)?;
            let h = m.ets_steps(&args[0], &series, cell)?;
            Ok(fit.forecast(series.values.len(), h))
        };
        run(self).map_or_else(|e| e, CalcResult::Number)
    }

    pub(crate) fn fn_forecast_ets_confint(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if !(3..=7).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let run = |m: &mut Self| -> Result<f64, CalcResult> {
            let confidence = match args.get(3) {
                None | Some(Node::EmptyArgKind) => 0.95,
                Some(a) => m.get_number(a, cell)?,
            };
            if !(confidence > 0.0 && confidence < 1.0) {
                return Err(CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "Invalid confidence level".to_string(),
                ));
            }
            let options = m.ets_options(args, Some(4), 5, cell)?;
            let (series, fit) = m.ets_fit(&args[1], &args[2], &options, cell)?;
            let h = m.ets_steps(&args[0], &series, cell)?;
            Ok(fit.interval(&series.values, h, confidence))
        };
        run(self).map_or_else(|e| e, CalcResult::Number)
    }

    pub(crate) fn fn_forecast_ets_seasonality(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if !(2..=4).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let run = |m: &mut Self| -> Result<f64, CalcResult> {
            let options = m.ets_options(args, None, 2, cell)?;
            let (_, fit) = m.ets_fit(&args[0], &args[1], &options, cell)?;
            Ok(fit.season as f64)
        };
        run(self).map_or_else(|e| e, CalcResult::Number)
    }

    pub(crate) fn fn_forecast_ets_stat(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if !(3..=6).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let run = |m: &mut Self| -> Result<f64, CalcResult> {
            let kind = m.get_number(&args[2], cell)?.trunc() as i32;
            let options = m.ets_options(args, Some(3), 4, cell)?;
            let (series, fit) = m.ets_fit(&args[0], &args[1], &options, cell)?;
            if kind == 8 {
                return Ok(series.step);
            }
            fit.statistic(&series.values, kind).ok_or_else(|| {
                CalcResult::new_error(Error::NUM, cell, "Invalid statistic".to_string())
            })
        };
        run(self).map_or_else(|e| e, CalcResult::Number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_series_from_an_unsorted_timeline_with_gaps_and_duplicates() {
        let s = series(
            &[3.0, 1.0, 2.0, 5.0, 2.0],
            &[Some(30.0), Some(10.0), Some(20.0), Some(50.0), Some(40.0)],
            true,
            1,
        )
        .unwrap();
        assert_eq!(s.values, vec![10.0, 30.0, 30.0, 40.0, 50.0]);
        assert_eq!((s.start, s.step), (1.0, 1.0));
        let zeros = series(
            &[1.0, 2.0, 4.0],
            &[Some(1.0), Some(2.0), Some(4.0)],
            false,
            7,
        )
        .unwrap();
        assert_eq!(zeros.values, vec![1.0, 2.0, 0.0, 4.0]);
        assert!(series(&[1.0, 2.0, 3.5], &[Some(1.0); 3], true, 1).is_err());
        assert!(series(&[1.0, 1.0, 1.0], &[Some(1.0); 3], true, 1).is_err());
    }

    #[test]
    fn a_seasonal_series_is_detected_and_forecast() {
        let pattern = [10.0, 14.0, 8.0, 12.0];
        let y: Vec<f64> = (0..24).map(|i| pattern[i % 4] + i as f64 * 0.5).collect();
        assert_eq!(detect_season(&y), 4);
        let f = fit(&y, 4);
        // The next point continues both the trend and the pattern.
        let next = f.forecast(y.len(), 1.0);
        assert!((next - (pattern[0] + 24.0 * 0.5)).abs() < 0.5, "{next}");
        assert!(f.statistic(&y, 7).unwrap() < 0.5);
    }

    #[test]
    fn a_linear_series_has_no_season_and_follows_its_trend() {
        let y: Vec<f64> = (0..12).map(|i| 3.0 + 2.0 * i as f64).collect();
        assert_eq!(detect_season(&y), 0);
        let f = fit(&y, 0);
        assert!((f.forecast(y.len(), 3.0) - (3.0 + 2.0 * 14.0)).abs() < 1e-6);
        assert!(f.interval(&y, 3.0, 0.95) < 1e-6);
    }
}
