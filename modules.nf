process BUILDINPUT {

    // prepares 3 files:
    // 1. a CSV file with the queries to be used by the rust reader (simply adds lower, upper headers to the queries CSV)
    // 2. a CSV file with the limits to be used by the cpp reader. Always builds a single bin.
    // 3. a EMBL etxt file containing entries for the uniprot accessions to be used by protgraph to build the bpcsr graph

    input:
    tuple(
        path(accessions_csv),
        path(queries_csv),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv),
        path(queries_csv),
        path("${prefix}_rust_queries.csv"),
        path("${prefix}_limits.csv"),
        path("${prefix}_uniprot.txt"),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    script:
    """
    python3 ${workflow.projectDir}/bin/python/prepare_inputs.py \\
        -i ${accessions_csv} \\
        -p ${prefix} \\
        -m ${max_vars} \\
        -q ${queries_csv}
    """
}

process BUILD_SYN_INPUT {
    // synthetic embl builder is called with uniprot only, as other sources can not be validated against uniprot.
    container 'sp-embl-builder'
    input:
    tuple(
        path(accessions_csv),
        path(queries_csv),
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv),
        path(queries_csv),
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path("${prefix}_syn_uniprot.txt"),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    script:
    """
    touch empty_variants.txt
    sp_embl_builder \
    --accessions ${accessions_csv} \
    --variants empty_variants.txt \
    --source-type uniprot \
    --output ${prefix}_syn_uniprot.txt

    """
}

process BUILDGRAPH {
    //builds a bpcsr graph using ProtGraph
    container 'quay.io/biocontainers/protgraph:0.3.12--pyhdfd78af_0'

    input:
    tuple(
        path(accessions_csv),
        path(queries_csv),
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        path(queries_csv),
        path(cpp_limits_csv),
        path(rust_queries_csv),
        path("*${prefix}/database.bpcsr"),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    script:
    """
    protgraph \\
        -elpcsr -elpcsr_pdbs 32 \\
        -elbpcsr -elbpcsr_pdbs 32 \\
        --pep_miscleavages ${max_cleavages} \\
        -nm \\
        -cnp \\
        -amw \\
        -ft VARIANT \\
        -d trypsin \\
        -eo ${prefix} \\
        ${uniprot_txt}
    """
}

process READERRUST {
    container 'fasta-builder-rust'
    input:
    tuple(
        path(queries_csv),
        path(cpp_limits_csv),
        path(rust_queries_csv),
        path(database_bpcsr),
        val(max_vars),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        path(database_bpcsr),
        path(queries_csv),
        path(cpp_limits_csv),
        path("./${prefix}_output/peptides.fasta"),
        val(max_cleavages),
        val(prefix)
    )

    script:
    p_count = Runtime.runtime.availableProcessors()
    """
    bpcsr_to_fasta \\
        --graphs ${database_bpcsr} \\
        --queries ${rust_queries_csv} \\
        --max_vars ${max_vars} \\
        --max_missed_cleavages ${max_cleavages} \\
        -o ./${prefix}_output \\
        --avail_processors ${p_count} \\
        --interval_bin_length 1000 \\
        --job_splits 1000 \\
        --split_depth 1\\
        --avail_memory 2 \\
        --hash_bits 3
    """
}

process READERCPP {
    container 'protgraph_reader_cpp'

    input:
    tuple(
        path(database_bpcsr),
        path(queries_csv),
        path(cpp_limits_csv),
        path(rust),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        path(rust),
        path("${prefix}_cpp_out.fasta"),
        val(max_cleavages),
        val(prefix)
    )

    script:
    p_count = Runtime.runtime.availableProcessors().div(2)
    """
    protgraphtraverseintvarlimitter \\
        ${database_bpcsr} \\
        ${queries_csv}\\
        ${p_count} \\
        ${prefix}_cpp_out.fasta \\
        ${cpp_limits_csv}
    """
}

process DEDUPCPP {
    // uses the protgraph_compact_fasta tool provided by ProtGraph to deduplicate the output of the cpp reader.
    container 'quay.io/biocontainers/protgraph:0.3.12--pyhdfd78af_0'

    input:
    tuple(
        path(rust),
        path(cpp_fasta),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        path(rust),
        path("dedup_${cpp_fasta}"),
        val(max_cleavages),
        val(prefix)
    )

    script:
    """
    protgraph_compact_fasta \\
        ${cpp_fasta} \\
        -o dedup_${cpp_fasta}
    """
}

process FILTERCLEAVAGES {

    // Filters the output of the cpp reader to only include peptides with a number of missed cleavages <= max_cleavages.
    // This is necessary because the cpp reader does not filter peptides based on the number of missed cleavages, while the rust reader does.

    input:
    tuple(
        path(rust),
        path(cpp),
        val(max_cleavages),
        val(prefix)
    )

    output:
    tuple(
        val(prefix),
        path(rust),
        path("filtered_${cpp}")
        
    )

    script:
    """
    paste - - < ${cpp} \
        | perl -ne 'chomp; my (\$header, \$seq) = split /\\t/; if (\$header =~ /mssclvg:(\\d+)/ && \$1 <= ${max_cleavages}) { print "\$header\\n\$seq\\n" }' \
        > filtered_${cpp}
    """
}

process DIFF {
    // Fasta headers are ignored, as they are not easily comparable.
    // Files are sorted before diffing, as the order of the peptides is not guaranteed to be the same between the two readers.

    publishDir("results/run_${prefix}/")

    input:
    tuple val(prefix), path(in1, stageAs: '1_peptides.fasta'), path(in2, stageAs: '2_peptides.fasta')

    output:
    tuple(
        path("sorted_${in1}"),
        path("sorted_${in2}"),
        path("${prefix}_diff.txt"),
    )

    script:
    """
    grep -v '^>' ${in1} | sort > sorted_${in1}
    grep -v '^>' ${in2} | sort > sorted_${in2}

    diff sorted_${in1} sorted_${in2} > ${prefix}_diff.txt || true
    """
}
