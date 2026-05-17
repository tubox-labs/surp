use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use surp_core::block::BlockReader;
use surp_core::checksum::compute_xxh64;
use surp_core::rfc001;
use surp_core::wire::{BlockType, CompressionType};
use surp_core::{Decoder, Encoder, Limits, Value};
use thiserror::Error;

type CliResult<T> = Result<T, CliError>;

#[derive(Parser, Debug)]
#[command(
    name = "surp",
    version,
    about = "Inspect, validate, and convert Surp data",
    long_about = "surp is a command-line tool for working with Surp binary files and text notation."
)]
struct Cli {
    /// Color mode for CLI output.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

    /// Suppress non-essential informational logs.
    #[arg(short, long, global = true, action = ArgAction::SetTrue)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect block layout, compression, and checksums.
    Inspect(InspectArgs),

    /// Pretty-print a .surp file as Surp text notation.
    Pretty(PrettyArgs),

    /// Convert a .surp file to JSON.
    ToJson(ToJsonArgs),

    /// Convert a JSON file to .surp binary.
    FromJson(FromJsonArgs),

    /// Parse Surp text notation and emit .surp binary.
    Encode(EncodeArgs),

    /// Decode .surp binary to Surp text notation.
    Decode(DecodeArgs),

    /// Validate checksums, trailer, and decode integrity.
    Validate(ValidateArgs),

    /// Benchmark encode/decode throughput from JSON input.
    Bench(BenchArgs),

    /// Compile RFC-001 CTN to RFC-001 CBF (.crb).
    RfcCompile(RfcCompileArgs),

    /// Inspect RFC-001 CBF header/symbol metadata and optionally decode CTN.
    RfcInspect(RfcInspectArgs),

    /// Execute a baseline RFC-001 CQL path query over a .crb file.
    RfcQuery(RfcQueryArgs),
}

#[derive(Args, Debug)]
struct InspectArgs {
    /// Input .surp file path (use '-' for stdin).
    file: PathBuf,
}

#[derive(Args, Debug)]
struct PrettyArgs {
    /// Input .surp file path (use '-' for stdin).
    file: PathBuf,

    /// Indentation width.
    #[arg(short, long, default_value_t = 2)]
    indent: usize,

    /// Output path (default: stdout).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum JsonStyle {
    Pretty,
    Compact,
}

#[derive(Args, Debug)]
struct ToJsonArgs {
    /// Input .surp file path (use '-' for stdin).
    file: PathBuf,

    /// JSON output style.
    #[arg(long, value_enum, default_value_t = JsonStyle::Pretty)]
    style: JsonStyle,

    /// Output path (default: stdout).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
enum CompressionArg {
    None,
    Lz4,
    Snappy,
    Zstd,
}

impl CompressionArg {
    fn to_wire(self) -> CompressionType {
        match self {
            CompressionArg::None => CompressionType::None,
            CompressionArg::Lz4 => CompressionType::Lz4,
            CompressionArg::Snappy => CompressionType::Snappy,
            CompressionArg::Zstd => CompressionType::Zstd,
        }
    }

    fn is_supported(self) -> bool {
        match self {
            CompressionArg::None => true,
            CompressionArg::Lz4 => cfg!(feature = "lz4"),
            CompressionArg::Snappy => cfg!(feature = "snappy"),
            CompressionArg::Zstd => cfg!(feature = "zstd"),
        }
    }
}

#[derive(Args, Debug)]
struct FromJsonArgs {
    /// Input JSON file path (use '-' for stdin).
    file: PathBuf,

    /// Output path (default: input path with .surp extension).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable per-block string deduplication.
    #[arg(long, action = ArgAction::SetTrue)]
    dedup: bool,

    /// Block compression for encoded output.
    #[arg(long, value_enum, default_value_t = CompressionArg::None, env = "SURP_COMPRESSION")]
    compression: CompressionArg,
}

#[derive(Args, Debug)]
struct EncodeArgs {
    /// Input Surp text file path (use '-' for stdin).
    file: PathBuf,

    /// Output path (default: input path with .surp extension).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable per-block string deduplication.
    #[arg(long, action = ArgAction::SetTrue)]
    dedup: bool,

    /// Block compression for encoded output.
    #[arg(long, value_enum, default_value_t = CompressionArg::None, env = "SURP_COMPRESSION")]
    compression: CompressionArg,
}

#[derive(Args, Debug)]
struct DecodeArgs {
    /// Input .surp file path (use '-' for stdin).
    file: PathBuf,

    /// Indentation width.
    #[arg(short, long, default_value_t = 2)]
    indent: usize,

    /// Output path (default: stdout).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    /// Input .surp file path (use '-' for stdin).
    file: PathBuf,

    /// Only validate framing/checksums/trailer; skip value decode.
    #[arg(long, action = ArgAction::SetTrue)]
    checksums_only: bool,

    /// Use strict safety limits during decode validation.
    #[arg(long, action = ArgAction::SetTrue)]
    strict: bool,
}

#[derive(Args, Debug)]
struct BenchArgs {
    /// Input JSON file path (use '-' for stdin).
    file: PathBuf,

    /// Number of measured iterations.
    #[arg(short = 'n', long, default_value = "1000")]
    iterations: NonZeroUsize,

    /// Warmup iterations (not included in measured results).
    #[arg(long, default_value_t = 100)]
    warmup: usize,

    /// Enable per-block string deduplication.
    #[arg(long, action = ArgAction::SetTrue)]
    dedup: bool,

    /// Block compression for benchmarked encoding.
    #[arg(long, value_enum, default_value_t = CompressionArg::None, env = "SURP_COMPRESSION")]
    compression: CompressionArg,
}

#[derive(Args, Debug)]
struct RfcCompileArgs {
    /// Input RFC-001 CTN file path (use '-' for stdin).
    file: PathBuf,

    /// Output path (default: input path with .crb extension).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Disable symbol table generation.
    #[arg(long = "no-symtab", action = ArgAction::SetTrue)]
    no_symtab: bool,

    /// Header alignment hint (0=no alignment, 4=16B, 6=64B, 7=128B).
    #[arg(long, default_value_t = 0)]
    alignment: u8,
}

#[derive(Args, Debug)]
struct RfcInspectArgs {
    /// Input RFC-001 CBF file path (use '-' for stdin).
    file: PathBuf,

    /// Decode and print CTN representation.
    #[arg(long, action = ArgAction::SetTrue)]
    ctn: bool,

    /// Output path for CTN when --ctn is used (default: stdout).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct RfcQueryArgs {
    /// Input RFC-001 CBF file path (use '-' for stdin).
    file: PathBuf,

    /// CQL expression (baseline path syntax).
    query: String,

    /// Output path (default: stdout).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Message(String),

    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },

    #[error("{context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("{context}: {source}")]
    Utf8 {
        context: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error(transparent)]
    Core(#[from] surp_core::SurpError),
}

impl CliError {
    fn message(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }
}

#[derive(Debug)]
struct Ui {
    color_enabled: bool,
    quiet: bool,
}

impl Ui {
    fn new(color: ColorChoice, quiet: bool) -> Self {
        let color_enabled = match color {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                std::env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal()
            }
        };
        Self {
            color_enabled,
            quiet,
        }
    }

    fn info(&self, msg: impl AsRef<str>) {
        if !self.quiet {
            eprintln!("{} {}", self.paint("34", "info"), msg.as_ref());
        }
    }

    fn success(&self, msg: impl AsRef<str>) {
        eprintln!("{} {}", self.paint("32", "ok"), msg.as_ref());
    }

    fn warn(&self, msg: impl AsRef<str>) {
        eprintln!("{} {}", self.paint("33", "warn"), msg.as_ref());
    }

    fn error(&self, msg: impl AsRef<str>) {
        eprintln!("{} {}", self.paint("31", "error"), msg.as_ref());
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

#[derive(Debug)]
struct BlockSummary {
    index: usize,
    offset: usize,
    block_type: BlockType,
    compression: CompressionType,
    payload_len: usize,
    payload_checksum_valid: Option<bool>,
}

#[derive(Debug)]
struct TrailerSummary {
    payload_checksum_valid: bool,
    file_checksum_valid: bool,
}

#[derive(Debug)]
struct InspectReport {
    file_size: usize,
    blocks: Vec<BlockSummary>,
    trailer: Option<TrailerSummary>,
}

#[derive(Copy, Clone)]
struct EncodeOptions {
    dedup: bool,
    compression: CompressionArg,
}

fn main() {
    let cli = Cli::parse();
    let ui = Ui::new(cli.color, cli.quiet);

    let result = match cli.command {
        Commands::Inspect(args) => cmd_inspect(&ui, &args),
        Commands::Pretty(args) => cmd_pretty(&ui, &args),
        Commands::ToJson(args) => cmd_to_json(&ui, &args),
        Commands::FromJson(args) => cmd_from_json(&ui, &args),
        Commands::Encode(args) => cmd_encode(&ui, &args),
        Commands::Decode(args) => cmd_decode(&ui, &args),
        Commands::Validate(args) => cmd_validate(&ui, &args),
        Commands::Bench(args) => cmd_bench(&ui, &args),
        Commands::RfcCompile(args) => cmd_rfc_compile(&ui, &args),
        Commands::RfcInspect(args) => cmd_rfc_inspect(&ui, &args),
        Commands::RfcQuery(args) => cmd_rfc_query(&ui, &args),
    };

    if let Err(err) = result {
        ui.error(err.to_string());
        std::process::exit(1);
    }
}

fn cmd_inspect(ui: &Ui, args: &InspectArgs) -> CliResult<()> {
    let data = read_input_bytes(&args.file)?;
    let report = analyze_blocks(&data)?;

    println!("File: {}", display_path(&args.file));
    println!("Size: {} bytes", report.file_size);
    println!();

    for block in &report.blocks {
        let checksum = match block.payload_checksum_valid {
            Some(true) => ui.paint("32", "ok"),
            Some(false) => ui.paint("31", "bad"),
            None => ui.paint("33", "n/a"),
        };
        println!(
            "#{:03} @ {:>8}  type={:?}  comp={:?}  payload={}B  checksum={}",
            block.index,
            block.offset,
            block.block_type,
            block.compression,
            block.payload_len,
            checksum,
        );
    }

    if let Some(trailer) = &report.trailer {
        println!();
        println!(
            "Trailer payload checksum: {}",
            if trailer.payload_checksum_valid {
                ui.paint("32", "ok")
            } else {
                ui.paint("31", "bad")
            }
        );
        println!(
            "Trailer file checksum:    {}",
            if trailer.file_checksum_valid {
                ui.paint("32", "ok")
            } else {
                ui.paint("31", "bad")
            }
        );
    } else {
        ui.warn("No trailer block found");
    }

    Ok(())
}

fn cmd_pretty(_ui: &Ui, args: &PrettyArgs) -> CliResult<()> {
    let data = read_input_bytes(&args.file)?;
    let mut decoder = Decoder::new(&data);
    let values = decoder.decode_all_owned()?;

    let text = values
        .iter()
        .map(|value| surp_core::text::pretty_print(value, args.indent))
        .collect::<Vec<_>>()
        .join("\n\n");

    write_text_output(args.output.as_ref(), text.as_bytes())
}

fn cmd_decode(ui: &Ui, args: &DecodeArgs) -> CliResult<()> {
    let pretty_args = PrettyArgs {
        file: args.file.clone(),
        indent: args.indent,
        output: args.output.clone(),
    };
    cmd_pretty(ui, &pretty_args)
}

fn cmd_rfc_compile(ui: &Ui, args: &RfcCompileArgs) -> CliResult<()> {
    let text = read_input_text(&args.file)?;
    let doc = rfc001::parse_document(&text)?;
    let bytes = rfc001::encode_document(
        &doc,
        rfc001::EncodeOptions {
            with_symtab: !args.no_symtab,
            alignment: args.alignment,
        },
    )?;

    let out_path = match &args.output {
        Some(path) => path.clone(),
        None => derive_output_path(&args.file, "crb")?,
    };

    write_binary_to_path(&out_path, &bytes)?;
    ui.success(format!(
        "Wrote {} bytes to {}",
        bytes.len(),
        out_path.display()
    ));
    Ok(())
}

fn cmd_rfc_inspect(_ui: &Ui, args: &RfcInspectArgs) -> CliResult<()> {
    let data = read_input_bytes(&args.file)?;
    let decoded = rfc001::decode_document(&data)?;

    println!("File: {}", display_path(&args.file));
    println!("Size: {} bytes", data.len());
    println!("Magic: {}", String::from_utf8_lossy(&rfc001::CBF_MAGIC));
    println!("CBF version: {}", decoded.header.cbf_version);
    println!("CTN version: {}", decoded.header.ctn_version);
    println!("Flags: 0x{:02x}", decoded.header.flags);
    println!("  self_describing: {}", decoded.header.self_describing());
    println!("  has_symtab:      {}", decoded.header.has_symtab());
    println!("  has_index:       {}", decoded.header.has_index());
    println!("Alignment hint: {}", decoded.header.alignment);
    println!("Root offset: {}", decoded.header.root_offset);
    println!("Symbol count: {}", decoded.symbols.len());

    if args.ctn {
        let text = rfc001::format_document(&decoded.document);
        write_text_output(args.output.as_ref(), text.as_bytes())?;
    }

    Ok(())
}

fn cmd_rfc_query(_ui: &Ui, args: &RfcQueryArgs) -> CliResult<()> {
    let data = read_input_bytes(&args.file)?;
    let decoded = rfc001::decode_document(&data)?;
    let root = decoded.document.effective_root()?;

    let results = rfc001::query(&root, &args.query)?;
    let rendered = if results.is_empty() {
        "null".to_string()
    } else if results.len() == 1 {
        rfc001::format_value(&results[0])
    } else {
        rfc001::format_value(&rfc001::Value::Sequence(rfc001::Sequence {
            elem_type: None,
            items: results,
        }))
    };

    write_text_output(args.output.as_ref(), rendered.as_bytes())
}

fn cmd_to_json(_ui: &Ui, args: &ToJsonArgs) -> CliResult<()> {
    let data = read_input_bytes(&args.file)?;
    let mut decoder = Decoder::new(&data);
    let values = decoder.decode_all_owned()?;

    let json_values: Vec<serde_json::Value> = values.iter().map(serde_json::Value::from).collect();

    let rendered = if json_values.len() == 1 {
        match args.style {
            JsonStyle::Pretty => serialize_json_pretty(&json_values[0], "Failed to render JSON")?,
            JsonStyle::Compact => serialize_json_compact(&json_values[0], "Failed to render JSON")?,
        }
    } else {
        match args.style {
            JsonStyle::Pretty => serialize_json_pretty(&json_values, "Failed to render JSON")?,
            JsonStyle::Compact => serialize_json_compact(&json_values, "Failed to render JSON")?,
        }
    };

    write_text_output(args.output.as_ref(), rendered.as_bytes())
}

fn cmd_from_json(ui: &Ui, args: &FromJsonArgs) -> CliResult<()> {
    let json_bytes = read_input_bytes(&args.file)?;
    let json_text = String::from_utf8(json_bytes).map_err(|source| CliError::Utf8 {
        context: format!(
            "Input '{}' is not valid UTF-8 JSON",
            display_path(&args.file)
        ),
        source,
    })?;

    let json_value: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|source| CliError::Json {
            context: format!("Failed to parse JSON from '{}'", display_path(&args.file)),
            source,
        })?;

    let surp_value = Value::from(&json_value);
    let bytes = encode_single_value(
        &surp_value,
        EncodeOptions {
            dedup: args.dedup,
            compression: args.compression,
        },
    )?;

    let out_path = match &args.output {
        Some(path) => path.clone(),
        None => derive_output_path(&args.file, "surp")?,
    };

    write_binary_to_path(&out_path, &bytes)?;

    let input_size = json_text.len();
    if input_size == 0 {
        ui.success(format!(
            "Wrote {} bytes to {}",
            bytes.len(),
            out_path.display()
        ));
    } else {
        let ratio = bytes.len() as f64 / input_size as f64;
        let delta = (1.0 - ratio) * 100.0;
        ui.success(format!(
            "Wrote {} bytes to {} ({:+.1}% vs JSON, {:.1}% of input)",
            bytes.len(),
            out_path.display(),
            delta,
            ratio * 100.0,
        ));
    }

    Ok(())
}

fn cmd_encode(ui: &Ui, args: &EncodeArgs) -> CliResult<()> {
    let text = read_input_text(&args.file)?;
    let value = surp_core::text::parse(&text)?;

    let bytes = encode_single_value(
        &value,
        EncodeOptions {
            dedup: args.dedup,
            compression: args.compression,
        },
    )?;

    let out_path = match &args.output {
        Some(path) => path.clone(),
        None => derive_output_path(&args.file, "surp")?,
    };

    write_binary_to_path(&out_path, &bytes)?;
    ui.success(format!(
        "Wrote {} bytes to {}",
        bytes.len(),
        out_path.display()
    ));
    Ok(())
}

fn cmd_validate(ui: &Ui, args: &ValidateArgs) -> CliResult<()> {
    let data = read_input_bytes(&args.file)?;
    let report = analyze_blocks(&data)?;

    let mut invalid_blocks = Vec::new();
    let mut compressed_blocks = Vec::new();

    for block in &report.blocks {
        if let Some(false) = block.payload_checksum_valid {
            invalid_blocks.push(block.index);
        }
        if block.payload_checksum_valid.is_none() {
            compressed_blocks.push(block.index);
        }
    }

    if !invalid_blocks.is_empty() {
        return Err(CliError::message(format!(
            "Invalid block checksum(s): {:?}",
            invalid_blocks
        )));
    }

    let trailer = report
        .trailer
        .ok_or_else(|| CliError::message("Missing trailer block"))?;

    if !trailer.payload_checksum_valid {
        return Err(CliError::message("Trailer payload checksum is invalid"));
    }
    if !trailer.file_checksum_valid {
        return Err(CliError::message("Trailer file checksum is invalid"));
    }

    if args.checksums_only {
        if !compressed_blocks.is_empty() {
            return Err(CliError::message(format!(
                "Cannot checksum-validate compressed block(s) {:?} without decode. Re-run without --checksums-only.",
                compressed_blocks
            )));
        }
        ui.success(format!(
            "Checksums valid for {} block(s)",
            report.blocks.len()
        ));
        return Ok(());
    }

    let limits = if args.strict {
        ui.info("Using strict decode limits");
        Limits::strict()
    } else {
        Limits::default()
    };

    let mut decoder = Decoder::with_limits(&data, limits);
    let values = decoder.decode_all_owned()?;
    ui.success(format!(
        "Validation passed: {} value(s) decoded, memory tracked {} bytes",
        values.len(),
        decoder.memory_used()
    ));
    Ok(())
}

fn cmd_bench(ui: &Ui, args: &BenchArgs) -> CliResult<()> {
    if !args.compression.is_supported() {
        return Err(unsupported_compression_error(args.compression));
    }

    let json_text = read_input_text(&args.file)?;
    let json_value: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|source| CliError::Json {
            context: format!("Failed to parse JSON from '{}'", display_path(&args.file)),
            source,
        })?;
    let surp_value = Value::from(&json_value);

    let iterations = args.iterations.get();
    for _ in 0..args.warmup {
        let mut encoder = Encoder::new();
        if args.dedup {
            encoder.enable_dedup();
        }
        encoder.set_compression(args.compression.to_wire());
        encoder.encode_value(&surp_value)?;
        let bytes = encoder.finish()?;

        let mut decoder = Decoder::new(&bytes);
        let _ = decoder.decode_all_owned()?;
    }

    let mut last_bytes = Vec::new();
    let encode_start = Instant::now();
    for _ in 0..iterations {
        let mut encoder = Encoder::new();
        if args.dedup {
            encoder.enable_dedup();
        }
        encoder.set_compression(args.compression.to_wire());
        encoder.encode_value(&surp_value)?;
        last_bytes = encoder.finish()?;
    }
    let encode_elapsed = encode_start.elapsed();

    let decode_start = Instant::now();
    for _ in 0..iterations {
        let mut decoder = Decoder::new(&last_bytes);
        let _ = decoder.decode_all_owned()?;
    }
    let decode_elapsed = decode_start.elapsed();

    let encode_secs = encode_elapsed.as_secs_f64().max(f64::EPSILON);
    let decode_secs = decode_elapsed.as_secs_f64().max(f64::EPSILON);
    let input_bytes = json_text.len();

    let encode_mbps = (input_bytes * iterations) as f64 / encode_secs / 1_000_000.0;
    let decode_mbps = (last_bytes.len() * iterations) as f64 / decode_secs / 1_000_000.0;
    let encode_avg = std::time::Duration::from_secs_f64(encode_secs / iterations as f64);
    let decode_avg = std::time::Duration::from_secs_f64(decode_secs / iterations as f64);

    println!("Benchmark: {}", display_path(&args.file));
    println!("Iterations: {} (warmup: {})", iterations, args.warmup);
    println!("Compression: {:?}", args.compression.to_wire());
    println!("Dedup: {}", args.dedup);
    println!("JSON size: {} bytes", input_bytes);
    println!("Surp size: {} bytes", last_bytes.len());

    println!(
        "Encode: total {:.2?}, avg {:.2?}, throughput {:.1} MB/s",
        encode_elapsed, encode_avg, encode_mbps
    );
    println!(
        "Decode: total {:.2?}, avg {:.2?}, throughput {:.1} MB/s",
        decode_elapsed, decode_avg, decode_mbps
    );
    ui.success("Benchmark complete");

    Ok(())
}

fn encode_single_value(value: &Value, options: EncodeOptions) -> CliResult<Vec<u8>> {
    if !options.compression.is_supported() {
        return Err(unsupported_compression_error(options.compression));
    }

    let mut encoder = Encoder::new();
    if options.dedup {
        encoder.enable_dedup();
    }
    encoder.set_compression(options.compression.to_wire());
    encoder.encode_value(value)?;
    encoder.finish().map_err(Into::into)
}

fn unsupported_compression_error(compression: CompressionArg) -> CliError {
    let feature = match compression {
        CompressionArg::None => return CliError::message("Compression 'none' is always supported"),
        CompressionArg::Lz4 => "lz4",
        CompressionArg::Snappy => "snappy",
        CompressionArg::Zstd => "zstd",
    };
    CliError::message(format!(
        "Compression '{feature}' is not enabled in this CLI build. Rebuild with `--features {feature}`."
    ))
}

fn analyze_blocks(data: &[u8]) -> CliResult<InspectReport> {
    let mut blocks = Vec::new();
    let mut trailer = None;

    let mut offset = 0usize;
    let mut index = 0usize;
    while offset < data.len() {
        let block_offset = offset;
        let (block, consumed) = BlockReader::parse(data, offset).map_err(|err| {
            CliError::message(format!(
                "Failed to parse block #{index} at offset {offset}: {err}"
            ))
        })?;
        offset += consumed;

        let payload_checksum_valid = if block.compression == CompressionType::None
            || block.block_type == BlockType::Trailer
        {
            Some(block.verify_checksum())
        } else {
            None
        };

        if block.block_type == BlockType::Trailer {
            let file_checksum_valid = if block.payload.len() == 8 {
                let mut expected_bytes = [0u8; 8];
                expected_bytes.copy_from_slice(block.payload);
                let expected = u64::from_le_bytes(expected_bytes);
                let actual = compute_xxh64(&data[..block_offset]);
                expected == actual
            } else {
                false
            };

            trailer = Some(TrailerSummary {
                payload_checksum_valid: block.verify_checksum(),
                file_checksum_valid,
            });
        }

        blocks.push(BlockSummary {
            index,
            offset: block_offset,
            block_type: block.block_type,
            compression: block.compression,
            payload_len: block.payload.len(),
            payload_checksum_valid,
        });
        index += 1;
    }

    Ok(InspectReport {
        file_size: data.len(),
        blocks,
        trailer,
    })
}

fn read_input_bytes(path: &Path) -> CliResult<Vec<u8>> {
    if is_stdio(path) {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|source| CliError::Io {
                context: "Failed to read from stdin".into(),
                source,
            })?;
        Ok(buf)
    } else {
        fs::read(path).map_err(|source| CliError::Io {
            context: format!("Failed to read '{}'", path.display()),
            source,
        })
    }
}

fn read_input_text(path: &Path) -> CliResult<String> {
    let bytes = read_input_bytes(path)?;
    String::from_utf8(bytes).map_err(|source| CliError::Utf8 {
        context: format!("Input '{}' is not valid UTF-8", display_path(path)),
        source,
    })
}

fn write_text_output(path: Option<&PathBuf>, text: &[u8]) -> CliResult<()> {
    match path {
        Some(output) if !is_stdio(output) => {
            fs::write(output, text).map_err(|source| CliError::Io {
                context: format!("Failed to write '{}'", output.display()),
                source,
            })
        }
        _ => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(text).map_err(|source| CliError::Io {
                context: "Failed to write to stdout".into(),
                source,
            })?;
            if !text.ends_with(b"\n") {
                stdout.write_all(b"\n").map_err(|source| CliError::Io {
                    context: "Failed to write trailing newline to stdout".into(),
                    source,
                })?;
            }
            Ok(())
        }
    }
}

fn write_binary_to_path(path: &Path, bytes: &[u8]) -> CliResult<()> {
    if is_stdio(path) {
        let mut stdout = io::stdout().lock();
        stdout.write_all(bytes).map_err(|source| CliError::Io {
            context: "Failed to write binary output to stdout".into(),
            source,
        })
    } else {
        fs::write(path, bytes).map_err(|source| CliError::Io {
            context: format!("Failed to write '{}'", path.display()),
            source,
        })
    }
}

fn derive_output_path(input: &Path, extension: &str) -> CliResult<PathBuf> {
    if is_stdio(input) {
        return Err(CliError::message(
            "Input is stdin ('-'); provide an explicit --output path",
        ));
    }
    Ok(input.with_extension(extension))
}

fn serialize_json_pretty<T: serde::Serialize>(value: &T, context: &str) -> CliResult<String> {
    serde_json::to_string_pretty(value).map_err(|source| CliError::Json {
        context: context.to_string(),
        source,
    })
}

fn serialize_json_compact<T: serde::Serialize>(value: &T, context: &str) -> CliResult<String> {
    serde_json::to_string(value).map_err(|source| CliError::Json {
        context: context.to_string(),
        source,
    })
}

fn is_stdio(path: &Path) -> bool {
    path == Path::new("-")
}

fn display_path(path: &Path) -> String {
    if is_stdio(path) {
        "stdin".to_string()
    } else {
        path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_output_path_works_for_regular_file() {
        let input = Path::new("data/input.json");
        let output = derive_output_path(input, "surp").expect("should derive output path");
        assert_eq!(output, PathBuf::from("data/input.surp"));
    }

    #[test]
    fn derive_output_path_rejects_stdin() {
        let input = Path::new("-");
        let err = derive_output_path(input, "surp").expect_err("stdin should fail");
        assert!(err.to_string().contains("stdin"));
    }

    #[test]
    fn analyze_blocks_detects_valid_trailer() {
        let mut encoder = Encoder::new();
        encoder
            .encode_value(&Value::Object(vec![("hello".into(), Value::UInt(42))]))
            .expect("encode should succeed");
        let bytes = encoder.finish().expect("finish should succeed");

        let report = analyze_blocks(&bytes).expect("analyze should succeed");
        assert!(!report.blocks.is_empty());
        let trailer = report.trailer.expect("trailer must exist");
        assert!(trailer.payload_checksum_valid);
        assert!(trailer.file_checksum_valid);
    }
}
