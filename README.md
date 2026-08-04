# variant-db-generation Module Validation Pipeline

Nextflow pipelines for validating components of [variant-db-generation](https://github.com/Sengenhoff-tim/variant-db-generation). 

The first pipeline compares the [**Rust**](https://github.com/Sengenhoff-tim/protgraph_bpcsr_reader) re-implementation against the original [**C++**](https://github.com/mpc-bioinformatics/ProGFASTAGen/blob/main/create_precursor_specific_fasta.nf) BPCSR reader for output files produced by [ProtGRaph](https://github.com/mpc-bioinformatics/ProtGraph)

The second pipeline compares peptide generation between a **UniProt**-sourced graph and a graph generated from **synthetic EMBL** files produced by [sp_embl_builder](https://github.com/Sengenhoff-tim/sp_embl_builder).

## Overview

- **`compare_bpcsr_readers`** — builds a single BPCSR graph from UniProt data,
  then runs both the Rust and C++ readers over it, filters/deduplicates their
  output, and diffs the resulting peptide sets against each other.
- **`compare_embl_files`** — builds two graphs (one from UniProt, one from a
  synthetic EMBL source) and diffs the peptides produced from each via the
  Rust reader.

## Requirements

- [Nextflow](https://www.nextflow.io/) (DSL2)
- Docker (or Singularity/Apptainer)
- Perl — used by the uncontainerized `FILTERCLEAVAGES` step
- Python 3 with the `requests` package — used by the uncontainerized
  `BUILDINPUT` step:
  ```bash
  pip install requests
  ```
- Standard Unix tools: `grep`, `sort`, `diff`, `paste` (present by default on
  most Linux systems)

### Docker images

| Image | Source | Notes |
|---|---|---|
| `quay.io/biocontainers/protgraph:0.3.12--pyhdfd78af_0` | Public registry | Auto-pulled, no setup needed |
| `sp-embl-builder` | Must be built locally | See below |
| `fasta-builder-rust` | Must be built locally | See below |
| `protgraph_reader_cpp` | Must be built locally | See below |

The three custom images are referenced by bare name (no registry prefix), so
Docker will **not** auto-pull them — they must be built before running the
pipeline.

```bash
docker build -t protgraph_reader_cpp -f bin/cpp/Dockerfile bin/cpp
docker build -t sp-embl-builder -f bin/sp_embl_builder/Dockerfile bin/sp_embl_builder
docker build -t fasta-builder-rust -f bin/fasta_builder_rust/Dockerfile bin/fasta_builder_rust
```

> **`protgraph_reader_cpp`** is compiled with `-march=native`, so it should be
> built on the same CPU architecture/machine where it will run — a binary
> built elsewhere may crash with an illegal instruction error.
>
> **`sp-embl-builder`** and **`fasta-builder-rust`** clone their source
> directly from GitHub during the build (no pinned commit/tag), so building
> requires network access, and rebuilding at a later date could pick up
> different source code than originally tested.

## Network requirements

This pipeline fetches data live from public APIs at runtime, 
internet access is required for every run, not just initial setup.

| Process | Runs on | External service(s) |
|---|---|---|
| `BUILDINPUT` | host (uncontainerized) | UniProt REST API |
| `BUILD_SYN_INPUT` | `sp-embl-builder` container | Ensembl, UniProt, EMBL-EBI |

Notes:
- `BUILDINPUT` fetches UniProt entries in batches (100 accessions per request
  by default) and has **no automatic retry**, a transient network error or
  UniProt rate-limiting will fail the run. Re-run with `-resume` once the
  issue clears.
- Docker containers must also be permitted outbound network access, since
  `BUILD_SYN_INPUT`'s fetches happen from inside the container.

## Input

Both subworkflows read a samplesheet CSV with the same format:

| Column | Description |
|---|---|
| `input_file` | Path to a CSV of UniProt accession IDs |
| `query_file` | Path to a CSV of query ranges (`lower,upper` per line) |
| `max_vars` | Max number of variants to consider |
| `max_cleavages` | Max number of missed cleavages |
| `run_prefix` | Prefix used to name/tag all outputs for this run |

Example:

```csv
input_file,query_file,max_vars,max_cleavages,run_prefix
accessions.csv,queries.csv,2,3,run1
```

## Usage

```bash
nextflow run main.nf --samplesheet samplesheet.csv --profile docker
```

Functional example samplesheets can be found in "run_params/samplesheets"

`--profile docker` is required.

## Output

Results are published to `results/run_<prefix>/`, containing:
- `sorted_<file1>`, `sorted_<file2>` — the sorted, header-stripped peptide lists that were compared
- `<prefix>_diff.txt` — the diff between the two peptide sets (empty if identical)