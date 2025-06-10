//! MPQ archive command implementations

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use std::fs;
use std::path::Path;
use wow_mpq::{
    Archive, ArchiveBuilder, FormatVersion, RebuildOptions,
    compare_archives as mpq_compare_archives, path::mpq_path_to_system, rebuild_archive,
};

use crate::utils::{
    NodeType, TreeNode, TreeOptions, add_table_row, create_progress_bar, create_spinner,
    create_table, detect_ref_type, format_bytes, format_compression_ratio, matches_pattern,
    render_tree, truncate_path,
};

#[derive(ValueEnum, Clone, Debug)]
pub enum VersionArg {
    V1,
    V2,
    V3,
    V4,
}

impl From<VersionArg> for FormatVersion {
    fn from(arg: VersionArg) -> Self {
        match arg {
            VersionArg::V1 => FormatVersion::V1,
            VersionArg::V2 => FormatVersion::V2,
            VersionArg::V3 => FormatVersion::V3,
            VersionArg::V4 => FormatVersion::V4,
        }
    }
}

#[derive(Subcommand)]
pub enum MpqCommands {
    /// List files in an MPQ archive
    List {
        /// Path to the MPQ archive
        archive: String,

        /// Show detailed information (size, compression ratio)
        #[arg(short, long)]
        long: bool,

        /// Filter files by pattern (supports wildcards)
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Extract files from an MPQ archive
    Extract {
        /// Path to the MPQ archive
        archive: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Specific files to extract (extracts all if not specified)
        files: Vec<String>,

        /// Preserve directory structure
        #[arg(short, long)]
        preserve_paths: bool,
    },

    /// Create a new MPQ archive
    Create {
        /// Path for the new MPQ archive
        archive: String,

        /// Files to add to the archive
        #[arg(short, long, required = true)]
        add: Vec<String>,

        /// Archive format version (v1, v2, v3, v4)
        #[arg(long, default_value = "v2")]
        version: String,

        /// Compression method (none, zlib, bzip2, lzma)
        #[arg(short, long, default_value = "zlib")]
        compression: String,

        /// Create or update (listfile)
        #[arg(long)]
        with_listfile: bool,
    },

    /// Show information about an MPQ archive
    Info {
        /// Path to the MPQ archive
        archive: String,

        /// Show hash table details
        #[arg(long)]
        show_hash_table: bool,

        /// Show block table details
        #[arg(long)]
        show_block_table: bool,
    },

    /// Validate integrity of an MPQ archive
    Validate {
        /// Path to the MPQ archive
        archive: String,

        /// Check CRC/MD5 checksums if available
        #[arg(long)]
        check_checksums: bool,
    },

    /// Rebuild an MPQ archive 1:1
    Rebuild {
        /// Source MPQ archive
        source: String,

        /// Target path for rebuilt archive
        target: String,

        /// Preserve original MPQ format version
        #[arg(long, default_value_t = true)]
        preserve_format: bool,

        /// Upgrade to specific format version
        #[arg(long, value_enum)]
        upgrade_to: Option<VersionArg>,

        /// Skip encrypted files
        #[arg(long)]
        skip_encrypted: bool,

        /// Skip digital signatures
        #[arg(long, default_value_t = true)]
        skip_signatures: bool,

        /// Verify rebuilt archive matches original
        #[arg(long)]
        verify: bool,

        /// Override compression method
        #[arg(long)]
        compression: Option<String>,

        /// Override block size
        #[arg(long)]
        block_size: Option<u16>,

        /// List files that would be processed (dry run)
        #[arg(long)]
        list_only: bool,
    },

    /// Compare two MPQ archives
    Compare {
        /// Source MPQ archive
        source: String,

        /// Target MPQ archive to compare against
        target: String,

        /// Show detailed file-by-file comparison
        #[arg(short, long)]
        detailed: bool,

        /// Compare actual file contents (slower but thorough)
        #[arg(long)]
        content_check: bool,

        /// Only compare archive metadata
        #[arg(long)]
        metadata_only: bool,

        /// Ignore file order differences
        #[arg(long)]
        ignore_order: bool,

        /// Output format: table, json, summary
        #[arg(long, default_value = "table")]
        output: String,

        /// Filter files by pattern (supports wildcards)
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Show tree structure of an MPQ archive
    Tree {
        /// Path to the MPQ archive
        archive: String,

        /// Maximum depth to display
        #[arg(long)]
        depth: Option<usize>,

        /// Hide external file references
        #[arg(long)]
        no_external_refs: bool,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Show compact metadata inline
        #[arg(long)]
        compact: bool,

        /// Filter files by pattern (supports wildcards)
        #[arg(short, long)]
        filter: Option<String>,
    },
}

pub fn execute(command: MpqCommands) -> Result<()> {
    match command {
        MpqCommands::List {
            archive,
            long,
            filter,
        } => list_archive(&archive, long, filter),
        MpqCommands::Extract {
            archive,
            output,
            files,
            preserve_paths,
        } => extract_files(&archive, &output, files, preserve_paths),
        MpqCommands::Create {
            archive,
            add,
            version,
            compression,
            with_listfile,
        } => create_archive(&archive, add, &version, &compression, with_listfile),
        MpqCommands::Info {
            archive,
            show_hash_table,
            show_block_table,
        } => show_info(&archive, show_hash_table, show_block_table),
        MpqCommands::Validate {
            archive,
            check_checksums,
        } => validate_archive(&archive, check_checksums),
        MpqCommands::Rebuild {
            source,
            target,
            preserve_format,
            upgrade_to,
            skip_encrypted,
            skip_signatures,
            verify,
            compression,
            block_size,
            list_only,
        } => rebuild_mpq_archive(RebuildParams {
            source_path: &source,
            target_path: &target,
            preserve_format,
            upgrade_to,
            skip_encrypted,
            skip_signatures,
            verify,
            compression,
            block_size,
            list_only,
        }),
        MpqCommands::Compare {
            source,
            target,
            detailed,
            content_check,
            metadata_only,
            ignore_order,
            output,
            filter,
        } => compare_archives(CompareParams {
            source_path: &source,
            target_path: &target,
            detailed,
            content_check,
            metadata_only,
            ignore_order,
            output_format: &output,
            filter,
        }),
        MpqCommands::Tree {
            archive,
            depth,
            no_external_refs,
            no_color,
            compact,
            filter,
        } => show_tree(
            &archive,
            depth,
            !no_external_refs,
            no_color,
            compact,
            filter,
        ),
    }
}

fn list_archive(path: &str, long: bool, filter: Option<String>) -> Result<()> {
    let spinner = create_spinner("Opening archive...");
    let mut archive = Archive::open(path).context("Failed to open archive")?;
    spinner.finish_and_clear();

    let files: Vec<String> = archive.list()?.into_iter().map(|e| e.name).collect();
    let pattern = filter.as_deref().unwrap_or("*");

    let mut filtered_files: Vec<_> = files
        .iter()
        .filter(|f| matches_pattern(f, pattern))
        .collect();
    filtered_files.sort();

    if filtered_files.is_empty() {
        println!("No files found matching pattern: {}", pattern);
        return Ok(());
    }

    if long {
        let mut table = create_table(vec!["File", "Size", "Compressed", "Ratio"]);

        for file in filtered_files {
            // Try to get file info from the entries
            if let Ok(entries) = archive.list() {
                if let Some(entry) = entries.iter().find(|e| &e.name == file) {
                    add_table_row(
                        &mut table,
                        vec![
                            truncate_path(file, 50),
                            format_bytes(entry.size),
                            format_bytes(entry.compressed_size),
                            format_compression_ratio(entry.size, entry.compressed_size),
                        ],
                    );
                }
            }
        }

        table.printstd();
    } else {
        for file in filtered_files {
            println!("{}", file);
        }
    }

    Ok(())
}

fn extract_files(
    archive_path: &str,
    output_dir: &str,
    files: Vec<String>,
    preserve_paths: bool,
) -> Result<()> {
    let spinner = create_spinner("Opening archive...");
    let mut archive = Archive::open(archive_path).context("Failed to open archive")?;
    spinner.finish_and_clear();

    let files_to_extract: Vec<String> = if files.is_empty() {
        archive.list()?.into_iter().map(|e| e.name).collect()
    } else {
        files
    };

    let pb = create_progress_bar(files_to_extract.len() as u64, "Extracting files");

    for file in &files_to_extract {
        pb.set_message(format!("Extracting: {}", file));

        match archive.read_file(file) {
            Ok(data) => {
                let output_path = if preserve_paths {
                    // Convert MPQ path separators to system path separators
                    let system_path = mpq_path_to_system(file);
                    Path::new(output_dir).join(system_path)
                } else {
                    // Convert MPQ path to system path, then extract just the filename
                    let system_path = mpq_path_to_system(file);
                    let filename = Path::new(&system_path).file_name().unwrap_or_default();
                    Path::new(output_dir).join(filename)
                };

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::write(&output_path, data)?;
            }
            Err(e) => {
                log::warn!("Failed to extract {}: {}", file, e);
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Extraction complete");
    Ok(())
}

fn create_archive(
    path: &str,
    files: Vec<String>,
    version: &str,
    compression: &str,
    with_listfile: bool,
) -> Result<()> {
    let mut builder = ArchiveBuilder::new();

    // Parse version
    let format_version = match version {
        "v1" => FormatVersion::V1,
        "v2" => FormatVersion::V2,
        "v3" => FormatVersion::V3,
        "v4" => FormatVersion::V4,
        _ => anyhow::bail!("Invalid version: {}", version),
    };
    builder = builder.version(format_version);

    // Parse compression
    let compression_flags = match compression {
        "none" => 0,
        "zlib" => wow_mpq::compression::flags::ZLIB,
        "bzip2" => wow_mpq::compression::flags::BZIP2,
        "lzma" => wow_mpq::compression::flags::LZMA,
        _ => anyhow::bail!("Invalid compression: {}", compression),
    };
    builder = builder.default_compression(compression_flags);

    if with_listfile {
        builder = builder.listfile_option(wow_mpq::ListfileOption::Generate);
    }

    let pb = create_progress_bar(files.len() as u64, "Adding files");

    for file_path in files {
        pb.set_message(format!("Adding: {}", file_path));
        let data = fs::read(&file_path)?;
        let archive_path = Path::new(&file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file_path);
        builder = builder.add_file_data(data, archive_path);
        pb.inc(1);
    }

    pb.finish_and_clear();

    let spinner = create_spinner("Building archive...");
    builder.build(path)?;
    spinner.finish_with_message("Archive created successfully");

    Ok(())
}

fn show_info(path: &str, show_hash_table: bool, show_block_table: bool) -> Result<()> {
    let spinner = create_spinner("Opening archive...");
    let mut archive = Archive::open(path).context("Failed to open archive")?;
    spinner.finish_and_clear();

    let info = archive.get_info()?;

    println!("MPQ Archive Information");
    println!("======================");
    println!("Path: {}", path);
    println!("Format version: {:?}", info.format_version);
    println!("Archive size: {}", format_bytes(info.file_size));
    println!("Number of files: {}", info.file_count);

    if show_hash_table {
        println!("\nHash Table:");
        println!("-----------");
        // Implementation would go here
        println!("(Hash table details not yet implemented)");
    }

    if show_block_table {
        println!("\nBlock Table:");
        println!("------------");
        // Implementation would go here
        println!("(Block table details not yet implemented)");
    }

    Ok(())
}

fn validate_archive(path: &str, check_checksums: bool) -> Result<()> {
    let spinner = create_spinner("Opening archive...");
    let mut archive = Archive::open(path).context("Failed to open archive")?;
    spinner.finish_and_clear();

    let files: Vec<String> = archive.list()?.into_iter().map(|e| e.name).collect();
    let pb = create_progress_bar(files.len() as u64, "Validating files");

    let mut errors = 0;

    for file in &files {
        pb.set_message(format!("Validating: {}", file));

        match archive.read_file(file) {
            Ok(_) => {
                // File read successfully
                if check_checksums {
                    // TODO: Implement checksum validation
                }
            }
            Err(e) => {
                log::error!("Failed to read {}: {}", file, e);
                errors += 1;
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    if errors == 0 {
        println!("✓ Archive validation passed");
    } else {
        println!("✗ Archive validation failed with {} errors", errors);
    }

    Ok(())
}

/// Parameters for MPQ archive rebuild operation
struct RebuildParams<'a> {
    source_path: &'a str,
    target_path: &'a str,
    preserve_format: bool,
    upgrade_to: Option<VersionArg>,
    skip_encrypted: bool,
    skip_signatures: bool,
    verify: bool,
    compression: Option<String>,
    block_size: Option<u16>,
    list_only: bool,
}

fn rebuild_mpq_archive(params: RebuildParams<'_>) -> Result<()> {
    // Parse compression override if provided
    let override_compression = if let Some(comp) = params.compression {
        let compression_flags = match comp.as_str() {
            "none" => 0,
            "zlib" => wow_mpq::compression::flags::ZLIB,
            "bzip2" => wow_mpq::compression::flags::BZIP2,
            "lzma" => wow_mpq::compression::flags::LZMA,
            _ => anyhow::bail!("Invalid compression: {}", comp),
        };
        Some(compression_flags)
    } else {
        None
    };

    // Set up rebuild options
    let options = RebuildOptions {
        preserve_format: params.preserve_format,
        target_format: params.upgrade_to.map(|v| v.into()),
        preserve_order: true,
        skip_encrypted: params.skip_encrypted,
        skip_signatures: params.skip_signatures,
        verify: params.verify,
        override_compression,
        override_block_size: params.block_size,
        list_only: params.list_only,
    };

    if params.list_only {
        println!("Analyzing source archive: {}", params.source_path);
    } else {
        println!(
            "Rebuilding archive: {} -> {}",
            params.source_path, params.target_path
        );
    }

    // Set up progress callback
    let progress_callback = Some(Box::new(|current: usize, total: usize, file: &str| {
        if current % 100 == 0 || current == total {
            println!("  [{}/{}] Processing: {}", current, total, file);
        }
    }) as Box<dyn Fn(usize, usize, &str) + Send + Sync>);

    // Perform the rebuild
    let spinner = if params.list_only {
        create_spinner("Analyzing archive...")
    } else {
        create_spinner("Rebuilding archive...")
    };

    let summary = rebuild_archive(
        params.source_path,
        params.target_path,
        options,
        progress_callback,
    )
    .context("Failed to rebuild archive")?;

    spinner.finish_and_clear();

    // Display results
    println!("\nRebuild Summary:");
    println!("================");
    println!("Source files: {}", summary.source_files);
    println!("Extracted files: {}", summary.extracted_files);
    if summary.skipped_files > 0 {
        println!("Skipped files: {}", summary.skipped_files);
    }
    println!("Target format: {:?}", summary.target_format);

    if params.list_only {
        println!("\nDry run completed. Use without --list-only to perform actual rebuild.");
    } else {
        if summary.verified {
            println!("✓ Verification: PASSED");
        } else if params.verify {
            println!("⚠ Verification: SKIPPED");
        }
        println!("✓ Archive rebuilt successfully: {}", params.target_path);
    }

    Ok(())
}

/// Parameters for MPQ archive comparison operation
struct CompareParams<'a> {
    source_path: &'a str,
    target_path: &'a str,
    detailed: bool,
    content_check: bool,
    metadata_only: bool,
    ignore_order: bool,
    output_format: &'a str,
    filter: Option<String>,
}

fn compare_archives(params: CompareParams<'_>) -> Result<()> {
    let spinner = create_spinner("Comparing archives...");

    let comparison_result = mpq_compare_archives(
        params.source_path,
        params.target_path,
        params.detailed,
        params.content_check,
        params.metadata_only,
        params.ignore_order,
        params.filter,
    )?;

    spinner.finish_and_clear();

    // Display results based on output format
    match params.output_format {
        "json" => {
            display_json_output(&comparison_result)?;
        }
        "summary" => {
            display_summary_output(params.source_path, params.target_path, &comparison_result)?;
        }
        "table" => {
            display_table_output(params.source_path, params.target_path, &comparison_result)?;
        }
        _ => {
            display_table_output(params.source_path, params.target_path, &comparison_result)?;
        }
    }

    Ok(())
}

fn display_summary_output(
    source_path: &str,
    target_path: &str,
    result: &wow_mpq::ComparisonResult,
) -> Result<()> {
    println!("Archive Comparison Summary");
    println!("=========================");
    println!("Source: {}", source_path);
    println!("Target: {}", target_path);
    println!();

    if result.identical {
        println!("✓ Archives are identical");
        return Ok(());
    }

    println!("✗ Archives differ");
    println!();

    // Metadata differences
    if !result.metadata.matches {
        println!("Metadata Differences:");
        if result.metadata.format_version.0 != result.metadata.format_version.1 {
            println!(
                "  Format Version: {:?} → {:?}",
                result.metadata.format_version.0, result.metadata.format_version.1
            );
        }
        if result.metadata.block_size.0 != result.metadata.block_size.1 {
            println!(
                "  Block Size: {} → {}",
                result.metadata.block_size.0, result.metadata.block_size.1
            );
        }
        if result.metadata.file_count.0 != result.metadata.file_count.1 {
            println!(
                "  File Count: {} → {}",
                result.metadata.file_count.0, result.metadata.file_count.1
            );
        }
        if result.metadata.archive_size.0 != result.metadata.archive_size.1 {
            println!(
                "  Archive Size: {} → {}",
                format_bytes(result.metadata.archive_size.0),
                format_bytes(result.metadata.archive_size.1)
            );
        }
        println!();
    }

    // File differences
    if let Some(files) = &result.files {
        if !files.source_only.is_empty() {
            println!(
                "Files only in source ({}): {}",
                files.source_only.len(),
                files
                    .source_only
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if files.source_only.len() > 5 {
                println!("  ... and {} more", files.source_only.len() - 5);
            }
            println!();
        }

        if !files.target_only.is_empty() {
            println!(
                "Files only in target ({}): {}",
                files.target_only.len(),
                files
                    .target_only
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if files.target_only.len() > 5 {
                println!("  ... and {} more", files.target_only.len() - 5);
            }
            println!();
        }

        if !files.size_differences.is_empty() {
            println!(
                "Files with size differences: {}",
                files.size_differences.len()
            );
        }

        if !files.content_differences.is_empty() {
            println!(
                "Files with content differences: {}",
                files.content_differences.len()
            );
        }

        if !files.metadata_differences.is_empty() {
            println!(
                "Files with metadata differences: {}",
                files.metadata_differences.len()
            );
        }
    }

    println!();
    println!(
        "Summary: {} total differences",
        result.summary.different_files
            + result.summary.source_only_count
            + result.summary.target_only_count
    );

    Ok(())
}

fn display_table_output(
    source_path: &str,
    target_path: &str,
    result: &wow_mpq::ComparisonResult,
) -> Result<()> {
    println!(
        "Archive Comparison: {} vs {}",
        truncate_path(source_path, 30),
        truncate_path(target_path, 30)
    );
    println!("{}", "=".repeat(80));

    if result.identical {
        println!("✓ Archives are identical");
        return Ok(());
    }

    // Metadata table
    let mut metadata_table = create_table(vec!["Property", "Source", "Target", "Match"]);
    add_table_row(
        &mut metadata_table,
        vec![
            "Format Version".to_string(),
            format!("{:?}", result.metadata.format_version.0),
            format!("{:?}", result.metadata.format_version.1),
            if result.metadata.format_version.0 == result.metadata.format_version.1 {
                "✓"
            } else {
                "✗"
            }
            .to_string(),
        ],
    );
    add_table_row(
        &mut metadata_table,
        vec![
            "Block Size".to_string(),
            result.metadata.block_size.0.to_string(),
            result.metadata.block_size.1.to_string(),
            if result.metadata.block_size.0 == result.metadata.block_size.1 {
                "✓"
            } else {
                "✗"
            }
            .to_string(),
        ],
    );
    add_table_row(
        &mut metadata_table,
        vec![
            "File Count".to_string(),
            result.metadata.file_count.0.to_string(),
            result.metadata.file_count.1.to_string(),
            if result.metadata.file_count.0 == result.metadata.file_count.1 {
                "✓"
            } else {
                "✗"
            }
            .to_string(),
        ],
    );
    add_table_row(
        &mut metadata_table,
        vec![
            "Archive Size".to_string(),
            format_bytes(result.metadata.archive_size.0),
            format_bytes(result.metadata.archive_size.1),
            if result.metadata.archive_size.0 == result.metadata.archive_size.1 {
                "✓"
            } else {
                "✗"
            }
            .to_string(),
        ],
    );

    println!("\nMetadata Comparison:");
    metadata_table.printstd();

    // File differences
    if let Some(files) = &result.files {
        if !files.source_only.is_empty()
            || !files.target_only.is_empty()
            || !files.size_differences.is_empty()
        {
            println!("\nFile Differences:");

            if !files.source_only.is_empty() {
                println!("\nFiles only in source ({}):", files.source_only.len());
                for file in files.source_only.iter().take(10) {
                    println!("  - {}", file);
                }
                if files.source_only.len() > 10 {
                    println!("  ... and {} more", files.source_only.len() - 10);
                }
            }

            if !files.target_only.is_empty() {
                println!("\nFiles only in target ({}):", files.target_only.len());
                for file in files.target_only.iter().take(10) {
                    println!("  + {}", file);
                }
                if files.target_only.len() > 10 {
                    println!("  ... and {} more", files.target_only.len() - 10);
                }
            }

            if !files.size_differences.is_empty() {
                println!("\nFiles with size differences:");
                let mut size_table =
                    create_table(vec!["File", "Source Size", "Target Size", "Compression"]);

                for diff in files.size_differences.iter().take(10) {
                    add_table_row(
                        &mut size_table,
                        vec![
                            truncate_path(&diff.name, 40),
                            format_bytes(diff.source_size),
                            format_bytes(diff.target_size),
                            format!(
                                "{} → {}",
                                format_compression_ratio(diff.source_size, diff.source_compressed),
                                format_compression_ratio(diff.target_size, diff.target_compressed)
                            ),
                        ],
                    );
                }

                size_table.printstd();

                if files.size_differences.len() > 10 {
                    println!(
                        "... and {} more files with size differences",
                        files.size_differences.len() - 10
                    );
                }
            }

            if !files.content_differences.is_empty() {
                println!(
                    "\nFiles with content differences ({}):",
                    files.content_differences.len()
                );
                for file in files.content_differences.iter().take(10) {
                    println!("  ≠ {}", file);
                }
                if files.content_differences.len() > 10 {
                    println!("  ... and {} more", files.content_differences.len() - 10);
                }
            }
        }
    }

    // Summary
    println!("\nSummary:");
    println!("  {} files identical", result.summary.identical_files);
    println!("  {} files different", result.summary.different_files);
    println!(
        "  {} files only in source",
        result.summary.source_only_count
    );
    println!(
        "  {} files only in target",
        result.summary.target_only_count
    );

    Ok(())
}

fn display_json_output(result: &wow_mpq::ComparisonResult) -> Result<()> {
    // For now, just pretty-print the debug representation
    // In a real implementation, you'd use serde_json
    println!("{:#?}", result);
    Ok(())
}

fn show_tree(
    path: &str,
    max_depth: Option<usize>,
    show_external_refs: bool,
    no_color: bool,
    compact: bool,
    filter: Option<String>,
) -> Result<()> {
    let spinner = create_spinner("Analyzing archive structure...");
    let mut archive = Archive::open(path).context("Failed to open archive")?;
    let info = archive.get_info()?;
    spinner.finish_and_clear();

    // Create root node with archive information
    let mut root = TreeNode::new(
        format!(
            "{}",
            std::path::Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
        ),
        NodeType::Root,
    )
    .with_size(info.file_size)
    .with_metadata("format", &format!("{:?}", info.format_version))
    .with_metadata("files", &info.file_count.to_string());

    // Add header information
    let header = TreeNode::new("Header".to_string(), NodeType::Header)
        .with_size(32) // Typical MPQ header size
        .with_metadata("version", &format!("{:?}", info.format_version))
        .with_metadata("sector_size", &format!("{}", info.sector_size));

    root = root.add_child(header);

    // Add hash table information
    if let Some(hash_table) = archive.hash_table() {
        let hash_node = TreeNode::new("Hash Table".to_string(), NodeType::Table)
            .with_size((hash_table.size() * 16) as u64) // Each hash entry is 16 bytes
            .with_metadata("entries", &hash_table.size().to_string())
            .with_metadata("encrypted", "true");
        root = root.add_child(hash_node);
    }

    // Add block table information
    if let Some(block_table) = archive.block_table() {
        let block_node = TreeNode::new("Block Table".to_string(), NodeType::Table)
            .with_size((block_table.size() * 16) as u64) // Each block entry is 16 bytes
            .with_metadata("entries", &block_table.size().to_string())
            .with_metadata("encrypted", "true");
        root = root.add_child(block_node);
    }

    // Add HET/BET tables if present
    if archive.het_table().is_some() {
        let het_node = TreeNode::new("HET Table".to_string(), NodeType::Table)
            .with_metadata("type", "Extended Hash Table")
            .with_metadata("version", "v4");
        root = root.add_child(het_node);
    }

    if archive.bet_table().is_some() {
        let bet_node = TreeNode::new("BET Table".to_string(), NodeType::Table)
            .with_metadata("type", "Extended Block Table")
            .with_metadata("version", "v4");
        root = root.add_child(bet_node);
    }

    // Build file tree
    let files: Vec<String> = archive.list()?.into_iter().map(|e| e.name).collect();
    let pattern = filter.as_deref().unwrap_or("*");
    let filtered_files: Vec<_> = files
        .iter()
        .filter(|f| matches_pattern(f, pattern))
        .collect();

    if !filtered_files.is_empty() {
        let mut files_node = TreeNode::new("Files".to_string(), NodeType::Directory)
            .with_metadata("count", &filtered_files.len().to_string());

        // Build directory structure
        let mut dir_structure = std::collections::BTreeMap::<String, Vec<&String>>::new();

        for file in &filtered_files {
            let path_parts: Vec<&str> = file.split('\\').collect();
            if path_parts.len() > 1 {
                let dir = path_parts[..path_parts.len() - 1].join("\\");
                dir_structure.entry(dir).or_default().push(file);
            } else {
                dir_structure.entry("/".to_string()).or_default().push(file);
            }
        }

        // Add directories and files to tree
        for (dir_path, dir_files) in dir_structure {
            if dir_path == "/" {
                // Root level files
                for file in dir_files {
                    let file_node = create_file_node(file, &mut archive, show_external_refs)?;
                    files_node = files_node.add_child(file_node);
                }
            } else {
                // Directory with files
                let mut dir_node = TreeNode::new(
                    format!("{}/", dir_path.split('\\').next_back().unwrap_or(&dir_path)),
                    NodeType::Directory,
                )
                .with_metadata("files", &dir_files.len().to_string());

                for file in dir_files {
                    let file_node = create_file_node(file, &mut archive, show_external_refs)?;
                    dir_node = dir_node.add_child(file_node);
                }

                files_node = files_node.add_child(dir_node);
            }
        }

        root = root.add_child(files_node);
    }

    // Add special files
    let special_files = vec!["(listfile)", "(attributes)", "(signature)"];
    for special_file in special_files {
        if archive.read_file(special_file).is_ok() {
            let special_node = match special_file {
                "(listfile)" => TreeNode::new("(listfile)".to_string(), NodeType::File)
                    .with_metadata("type", "Auto-generated file list")
                    .with_metadata("purpose", "File enumeration"),
                "(attributes)" => TreeNode::new("(attributes)".to_string(), NodeType::File)
                    .with_metadata("type", "File attributes")
                    .with_metadata("purpose", "CRC checksums and timestamps"),
                "(signature)" => TreeNode::new("(signature)".to_string(), NodeType::File)
                    .with_metadata("type", "Digital signature")
                    .with_metadata("purpose", "Archive integrity verification"),
                _ => continue,
            };
            root = root.add_child(special_node);
        }
    }

    // Render the tree
    let options = TreeOptions {
        max_depth,
        show_external_refs,
        no_color,
        show_metadata: true,
        compact,
    };

    println!("{}", render_tree(&root, &options));
    Ok(())
}

fn create_file_node(
    file_path: &str,
    archive: &mut Archive,
    show_external_refs: bool,
) -> Result<TreeNode> {
    let file_name = file_path.split('\\').next_back().unwrap_or(file_path);
    let extension = std::path::Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut node = TreeNode::new(file_name.to_string(), NodeType::File);

    // Add file size if available
    if let Ok(entries) = archive.list() {
        if let Some(entry) = entries.iter().find(|e| e.name == file_path) {
            node = node
                .with_size(entry.size)
                .with_metadata("compressed_size", &format_bytes(entry.compressed_size))
                .with_metadata(
                    "compression_ratio",
                    &format_compression_ratio(entry.size, entry.compressed_size),
                );
        }
    }

    // Add file type metadata
    let file_type = match extension.to_lowercase().as_str() {
        "blp" => "Texture",
        "m2" | "mdx" => "Model",
        "wdt" => "World Map Definition",
        "adt" => "Terrain Data",
        "wdl" => "Low-res Terrain",
        "dbc" => "Database",
        "lua" => "Script",
        "xml" => "Interface Definition",
        "toc" => "AddOn Manifest",
        "wav" | "mp3" => "Audio",
        _ => "Data",
    };
    node = node.with_metadata("type", file_type);

    // Add external references for certain file types
    if show_external_refs {
        match extension.to_lowercase().as_str() {
            "wdt" => {
                // WDT files reference ADT files
                let base_name = file_name.trim_end_matches(".wdt");
                node = node.with_external_ref(
                    &format!("{}/*.adt", base_name),
                    detect_ref_type("file.adt"),
                );
            }
            "adt" => {
                // ADT files might reference textures and models
                node = node.with_external_ref("*.blp", detect_ref_type("file.blp"));
                node = node.with_external_ref("*.m2", detect_ref_type("file.m2"));
            }
            "m2" => {
                // M2 files reference textures and animations
                let base_name = file_name.trim_end_matches(".m2");
                node = node.with_external_ref(
                    &format!("{}.skin", base_name),
                    detect_ref_type("file.skin"),
                );
                node = node.with_external_ref("*.blp", detect_ref_type("file.blp"));
            }
            "dbc" => {
                // Some DBC files reference other files
                if file_name.to_lowercase().contains("item") {
                    node = node
                        .with_external_ref("Interface/Icons/*.blp", detect_ref_type("file.blp"));
                }
            }
            _ => {}
        }
    }

    Ok(node)
}
