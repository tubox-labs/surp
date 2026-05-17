//! Report generation: raw JSON, summary CSV, regression Markdown.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::metrics::{BenchReport, Measurement, Regression, Severity};

/// Write all output artifacts to the given directory.
pub fn write_all(
    dir: &Path,
    report: &BenchReport,
    regressions: &[Regression],
    baseline: Option<&BenchReport>,
) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;

    // 1. raw.json
    let raw_json = serde_json::to_string_pretty(report).unwrap();
    fs::write(dir.join("raw.json"), &raw_json)?;

    // 2. system_info.json
    let sys_json = serde_json::to_string_pretty(&report.system).unwrap();
    fs::write(dir.join("system_info.json"), &sys_json)?;

    // 3. summary.csv
    write_csv(dir, &report.measurements)?;

    // 4. regression_report.md
    write_regression_md(dir, report, regressions, baseline)?;

    // 5. size_comparison.md
    write_size_comparison(dir, &report.measurements)?;

    // 6. SVG charts for release documentation.
    write_charts(dir, report)?;

    Ok(())
}

fn write_csv(dir: &Path, measurements: &[Measurement]) -> std::io::Result<()> {
    let path = dir.join("summary.csv");
    let mut wtr = csv::Writer::from_path(&path)?;

    wtr.write_record([
        "format",
        "dataset",
        "operation",
        "median_ns",
        "mean_ns",
        "p95_ns",
        "p99_ns",
        "min_ns",
        "max_ns",
        "stddev_ns",
        "cv_pct",
        "throughput_mbps",
        "serialized_size",
        "runs",
    ])?;

    for m in measurements {
        wtr.write_record([
            &m.format,
            &m.dataset,
            &m.operation,
            &m.median_ns().to_string(),
            &format!("{:.0}", m.mean_ns()),
            &m.p95_ns().to_string(),
            &m.p99_ns().to_string(),
            &m.min_ns().to_string(),
            &m.max_ns().to_string(),
            &format!("{:.0}", m.stddev_ns()),
            &format!("{:.2}", m.cv() * 100.0),
            &m.throughput_mbps()
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "-".into()),
            &m.serialized_size
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".into()),
            &m.durations_ns.len().to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

fn write_regression_md(
    dir: &Path,
    report: &BenchReport,
    regressions: &[Regression],
    baseline: Option<&BenchReport>,
) -> std::io::Result<()> {
    let path = dir.join("regression_report.md");
    let mut f = fs::File::create(&path)?;

    writeln!(f, "# Surp Regression Benchmark Report")?;
    writeln!(f)?;
    writeln!(f, "**Version:** `{}`", report.version)?;
    writeln!(f, "**Timestamp:** {}", report.timestamp)?;
    writeln!(f, "**Mode:** {}", report.mode)?;
    writeln!(f, "**Dataset version:** {}", report.dataset_version)?;
    writeln!(f)?;

    // System info.
    writeln!(f, "## System")?;
    writeln!(f)?;
    writeln!(f, "| Property | Value |")?;
    writeln!(f, "|----------|-------|")?;
    writeln!(f, "| OS | {} | ", report.system.os)?;
    writeln!(f, "| Arch | {} |", report.system.arch)?;
    writeln!(f, "| CPU | {} |", report.system.cpu_model)?;
    writeln!(f, "| Cores | {} |", report.system.cpu_cores)?;
    writeln!(f, "| RAM | {} MB |", report.system.ram_mb)?;
    writeln!(f, "| Rust | {} |", report.system.rust_version)?;
    writeln!(f)?;

    // Overall status.
    let failures = regressions
        .iter()
        .filter(|r| r.severity == Severity::Failure)
        .count();
    let warnings = regressions
        .iter()
        .filter(|r| r.severity == Severity::Warning)
        .count();

    if failures > 0 {
        writeln!(f, "## REGRESSION DETECTED")?;
        writeln!(f)?;
        writeln!(f, "**{failures} failure(s), {warnings} warning(s)**")?;
    } else if warnings > 0 {
        writeln!(f, "## WARNINGS")?;
        writeln!(f)?;
        writeln!(f, "**{warnings} warning(s), 0 failures**")?;
    } else {
        writeln!(f, "## NO REGRESSIONS DETECTED")?;
    }
    writeln!(f)?;

    // Regression details.
    if !regressions.is_empty() {
        writeln!(f, "### Regression Details")?;
        writeln!(f)?;
        writeln!(
            f,
            "| Severity | Format | Dataset | Metric | Baseline | Current | Change | Threshold |"
        )?;
        writeln!(
            f,
            "|----------|--------|---------|--------|----------|---------|--------|-----------|"
        )?;
        for r in regressions {
            let label = match r.severity {
                Severity::Failure => "FAIL",
                Severity::Warning => "WARN",
            };
            writeln!(
                f,
                "| {} | {} | {} | {} | {:.0} | {:.0} | {:+.1}% | {:.0}% |",
                label,
                r.format,
                r.dataset,
                r.metric,
                r.baseline_value,
                r.current_value,
                r.change_pct,
                r.threshold_pct,
            )?;
        }
        writeln!(f)?;
    }

    // Performance summary table.
    writeln!(f, "## Performance Summary")?;
    writeln!(f)?;
    writeln!(
        f,
        "| Format | Dataset | Op | Median (us) | p95 (us) | CV% | MB/s | Size |"
    )?;
    writeln!(
        f,
        "|--------|---------|-----|-------------|----------|-----|------|------|"
    )?;

    for m in &report.measurements {
        let median_us = m.median_ns() as f64 / 1000.0;
        let p95_us = m.p95_ns() as f64 / 1000.0;
        let mbps = m
            .throughput_mbps()
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "-".into());
        let size = m
            .serialized_size
            .map(format_bytes)
            .unwrap_or_else(|| "-".into());

        writeln!(
            f,
            "| {} | {} | {} | {:.1} | {:.1} | {:.1} | {} | {} |",
            m.format,
            m.dataset,
            m.operation,
            median_us,
            p95_us,
            m.cv() * 100.0,
            mbps,
            size,
        )?;
    }
    writeln!(f)?;

    // Baseline comparison if available.
    if let Some(base) = baseline {
        writeln!(f, "## Baseline Comparison")?;
        writeln!(f)?;
        writeln!(f, "**Baseline version:** `{}`", base.version)?;
        writeln!(f, "**Baseline timestamp:** {}", base.timestamp)?;
    }

    writeln!(f)?;
    writeln!(
        f,
        "---\n*Generated by surp-bench v{} on {}*",
        env!("CARGO_PKG_VERSION"),
        report.timestamp,
    )?;

    Ok(())
}

fn write_size_comparison(dir: &Path, measurements: &[Measurement]) -> std::io::Result<()> {
    let path = dir.join("size_comparison.md");
    let mut f = fs::File::create(&path)?;

    writeln!(f, "# Size Comparison")?;
    writeln!(f)?;
    writeln!(
        f,
        "| Dataset | Surp | Surp+Dedup | JSON | MsgPack | CBOR | Protobuf | Surp/JSON |"
    )?;
    writeln!(
        f,
        "|---------|-------|-------------|------|---------|------|----------|------------|"
    )?;

    // Collect encode sizes per dataset.
    let datasets: Vec<&str> = measurements
        .iter()
        .filter(|m| m.operation == "encode" && m.format == "surp")
        .map(|m| m.dataset.as_str())
        .collect();

    for ds in &datasets {
        let get_size = |fmt: &str| -> String {
            measurements
                .iter()
                .find(|m| m.dataset == *ds && m.format == fmt && m.operation == "encode")
                .and_then(|m| m.serialized_size)
                .map(format_bytes)
                .unwrap_or_else(|| "-".into())
        };
        let get_size_raw = |fmt: &str| -> Option<usize> {
            measurements
                .iter()
                .find(|m| m.dataset == *ds && m.format == fmt && m.operation == "encode")
                .and_then(|m| m.serialized_size)
        };

        let ratio = match (get_size_raw("surp"), get_size_raw("json")) {
            (Some(c), Some(j)) if j > 0 => format!("{:.2}x", c as f64 / j as f64),
            _ => "-".into(),
        };

        writeln!(
            f,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            ds,
            get_size("surp"),
            get_size("surp_dedup"),
            get_size("json"),
            get_size("msgpack"),
            get_size("cbor"),
            get_size("protobuf"),
            ratio,
        )?;
    }

    Ok(())
}

fn write_charts(dir: &Path, report: &BenchReport) -> std::io::Result<()> {
    let chart_dir = dir.join("charts");
    fs::create_dir_all(&chart_dir)?;

    write_size_chart(&chart_dir.join("serialized-size.svg"), &report.measurements)?;
    write_throughput_chart(
        &chart_dir.join("encode-throughput.svg"),
        &report.measurements,
        "encode",
        "Median encode throughput by dataset",
    )?;
    write_throughput_chart(
        &chart_dir.join("decode-throughput.svg"),
        &report.measurements,
        "decode",
        "Median decode throughput by dataset",
    )?;

    Ok(())
}

fn write_size_chart(path: &Path, measurements: &[Measurement]) -> std::io::Result<()> {
    let formats = ["surp", "surp_dedup", "json", "msgpack", "protobuf"];
    let datasets = sorted_datasets(measurements);
    let values = collect_metric(measurements, &datasets, &formats, "encode", |m| {
        m.serialized_size.map(|value| value as f64)
    });
    write_grouped_bar_svg(
        ChartSpec {
            path,
            title: "Serialized size by dataset",
            subtitle: "Lower is better. Bytes, log10 scale.",
            datasets: &datasets,
            formats: &formats,
            values: &values,
            scale: Scale::Log10,
        },
        |value| format_bytes(value as usize),
    )
}

fn write_throughput_chart(
    path: &Path,
    measurements: &[Measurement],
    operation: &str,
    title: &str,
) -> std::io::Result<()> {
    let formats = ["surp", "json", "msgpack", "protobuf"];
    let datasets = sorted_datasets(measurements);
    let values = collect_metric(measurements, &datasets, &formats, operation, |m| {
        m.throughput_mbps()
    });
    write_grouped_bar_svg(
        ChartSpec {
            path,
            title,
            subtitle: "Higher is better. MB/s, log10 scale.",
            datasets: &datasets,
            formats: &formats,
            values: &values,
            scale: Scale::Log10,
        },
        |value| format!("{value:.0} MB/s"),
    )
}

fn sorted_datasets(measurements: &[Measurement]) -> Vec<String> {
    let mut datasets: Vec<String> = measurements
        .iter()
        .filter(|m| m.operation == "encode" && m.format == "surp")
        .map(|m| m.dataset.clone())
        .collect();
    datasets.sort();
    datasets
}

fn collect_metric(
    measurements: &[Measurement],
    datasets: &[String],
    formats: &[&str],
    operation: &str,
    extract: impl Fn(&Measurement) -> Option<f64>,
) -> Vec<Vec<Option<f64>>> {
    datasets
        .iter()
        .map(|dataset| {
            formats
                .iter()
                .map(|format| {
                    measurements
                        .iter()
                        .find(|m| {
                            m.dataset == *dataset && m.format == *format && m.operation == operation
                        })
                        .and_then(&extract)
                })
                .collect()
        })
        .collect()
}

#[derive(Copy, Clone)]
enum Scale {
    Log10,
}

struct ChartSpec<'a> {
    path: &'a Path,
    title: &'a str,
    subtitle: &'a str,
    datasets: &'a [String],
    formats: &'a [&'a str],
    values: &'a [Vec<Option<f64>>],
    scale: Scale,
}

fn write_grouped_bar_svg(
    spec: ChartSpec<'_>,
    format_value: impl Fn(f64) -> String,
) -> std::io::Result<()> {
    let width = 1200.0;
    let height = 720.0;
    let margin_left = 88.0;
    let margin_right = 42.0;
    let margin_top = 92.0;
    let margin_bottom = 150.0;
    let plot_width = width - margin_left - margin_right;
    let plot_height = height - margin_top - margin_bottom;

    let max_value = spec
        .values
        .iter()
        .flat_map(|row| row.iter().flatten())
        .copied()
        .fold(0.0, f64::max)
        .max(1.0);
    let scaled_max = scale_value(max_value, spec.scale);

    let palette = [
        ("surp", "#276EF1"),
        ("surp_dedup", "#00A676"),
        ("json", "#E4572E"),
        ("msgpack", "#7A5CFA"),
        ("protobuf", "#F4A261"),
        ("cbor", "#4C6B73"),
    ];

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{}">"#,
        escape_xml(spec.title)
    ));
    svg.push_str(
        r##"<rect width="100%" height="100%" fill="#ffffff"/><style>text{font-family:Inter,Arial,sans-serif;fill:#17202a}.title{font-size:28px;font-weight:700}.subtitle{font-size:14px;fill:#5d6d7e}.axis{font-size:12px;fill:#34495e}.label{font-size:11px;fill:#34495e}.legend{font-size:13px;fill:#17202a}</style>"##,
    );
    svg.push_str(&format!(
        r#"<text class="title" x="{margin_left}" y="42">{}</text>"#,
        escape_xml(spec.title)
    ));
    svg.push_str(&format!(
        r#"<text class="subtitle" x="{margin_left}" y="66">{}</text>"#,
        escape_xml(spec.subtitle)
    ));

    for tick in 0..=5 {
        let y = margin_top + plot_height - (plot_height * tick as f64 / 5.0);
        svg.push_str(&format!(
            r##"<line x1="{margin_left}" y1="{y:.1}" x2="{}" y2="{y:.1}" stroke="#e7edf3" stroke-width="1"/>"##,
            width - margin_right
        ));
        let tick_scaled = scaled_max * tick as f64 / 5.0;
        let tick_value = unscale_value(tick_scaled, spec.scale);
        svg.push_str(&format!(
            r#"<text class="axis" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            margin_left - 10.0,
            y + 4.0,
            escape_xml(&format_value(tick_value))
        ));
    }

    svg.push_str(&format!(
        r##"<line x1="{margin_left}" y1="{margin_top}" x2="{margin_left}" y2="{}" stroke="#93a4b7" stroke-width="1.2"/>"##,
        margin_top + plot_height
    ));
    svg.push_str(&format!(
        r##"<line x1="{margin_left}" y1="{}" x2="{}" y2="{}" stroke="#93a4b7" stroke-width="1.2"/>"##,
        margin_top + plot_height,
        width - margin_right,
        margin_top + plot_height
    ));

    let group_width = plot_width / spec.datasets.len().max(1) as f64;
    let inner_gap = 4.0;
    let bar_width = ((group_width * 0.76) / spec.formats.len().max(1) as f64 - inner_gap).max(2.0);

    for (dataset_index, dataset) in spec.datasets.iter().enumerate() {
        let group_x = margin_left + dataset_index as f64 * group_width + group_width * 0.12;
        for (format_index, format) in spec.formats.iter().enumerate() {
            let Some(value) = spec.values[dataset_index][format_index] else {
                continue;
            };
            let scaled = scale_value(value, spec.scale);
            let bar_height = if scaled_max == 0.0 {
                0.0
            } else {
                plot_height * (scaled / scaled_max)
            };
            let x = group_x + format_index as f64 * (bar_width + inner_gap);
            let y = margin_top + plot_height - bar_height;
            let color = palette
                .iter()
                .find(|(name, _)| name == format)
                .map(|(_, color)| *color)
                .unwrap_or("#7f8c8d");
            svg.push_str(&format!(
                r##"<rect x="{x:.1}" y="{y:.1}" width="{bar_width:.1}" height="{bar_height:.1}" fill="{color}" rx="2"><title>{}: {} {}</title></rect>"##,
                escape_xml(dataset),
                escape_xml(format),
                escape_xml(&format_value(value))
            ));
        }

        let label_x = margin_left + dataset_index as f64 * group_width + group_width * 0.5;
        svg.push_str(&format!(
            r#"<text class="label" x="{label_x:.1}" y="{}" text-anchor="end" transform="rotate(-35 {label_x:.1} {})">{}</text>"#,
            margin_top + plot_height + 36.0,
            margin_top + plot_height + 36.0,
            escape_xml(dataset)
        ));
    }

    let mut legend_x = margin_left;
    let legend_y = height - 28.0;
    for format in spec.formats {
        let color = palette
            .iter()
            .find(|(name, _)| name == format)
            .map(|(_, color)| *color)
            .unwrap_or("#7f8c8d");
        let label = match *format {
            "surp_dedup" => "surp+dedup",
            other => other,
        };
        svg.push_str(&format!(
            r##"<rect x="{legend_x:.1}" y="{:.1}" width="14" height="14" fill="{color}" rx="2"/><text class="legend" x="{:.1}" y="{:.1}">{}</text>"##,
            legend_y - 12.0,
            legend_x + 20.0,
            legend_y,
            escape_xml(label)
        ));
        legend_x += 132.0;
    }

    svg.push_str("</svg>\n");
    fs::write(spec.path, svg)
}

fn scale_value(value: f64, scale: Scale) -> f64 {
    match scale {
        Scale::Log10 => value.max(1.0).log10(),
    }
}

fn unscale_value(value: f64, scale: Scale) -> f64 {
    match scale {
        Scale::Log10 => 10f64.powf(value),
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
