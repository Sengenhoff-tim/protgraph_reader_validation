params.samplesheet = "${workflow.projectDir}/run_params/samplesheets/samplesheet15.csv"

process BUILDINPUT {

    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path("${prefix}_rust_queries.csv"),
        path("${prefix}_limits.csv"),
        path("${prefix}_uniprot.txt"),
        path("${prefix}_summary.txt"),
        val(max_vars), 
        val(prefix)
    )

    script:
    """
    python3 ${workflow.projectDir}/bin/python/prepare_inputs.py \\
        -i ${accessions_csv} \\
        -p ${prefix} \\
        -m ${max_vars} \\
        -q ${queries_csv}

    echo "${prefix}, ${accessions_csv}, ${max_vars}" > ${prefix}_summary.txt

    """
}

process BUILDGRAPH {
    container 'quay.io/biocontainers/protgraph:0.3.12--pyhdfd78af_0'

    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path("*${prefix}/database.bpcsr"),
        val(max_vars), 
        val(prefix)
    )

    script:
    """
    protgraph \\
        -elpcsr -elpcsr_pdbs 32 \\
        -elbpcsr -elbpcsr_pdbs 32 \\
        --pep_miscleavages 3 \\
        -nm \\
        -cnp \\
        -amw \\
        -ft ALL \\
        -d trypsin \\
        -eo ${prefix} \\
        ${uniprot_txt}
    """
}
//-ft VARIANT -ft SIGNAL -ft INIT_MET -ft CONFLICT -ft VAR_SEQ -ft PEPTIDE -ft PROPEP -ft CHAIN

process READERRUST {
    container 'protgraph_reader_rust'
    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path("./${prefix}_output/peptides.fasta.gz"),
        val(max_vars), 
        val(prefix)
    )

    script:
    p_count = Runtime.runtime.availableProcessors()
    """
    bpcsr_to_fasta \\
        --graphs ${database_bpcsr} \\
        --queries ${rust_queries_csv} \\
        --max_vars 3 \\
        -o ./${prefix}_output \\
        --avail_processors ${p_count} \\
        -i 100 \\
        --avail_memory 8 \\
        --hash_bits 3 \\
        -z
    """
}

process READERCPP {
    container 'protgraph_reader_cpp'

    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path(rust),
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path(rust),
        path("${prefix}_cpp_out.fasta"),
        val(max_vars), 
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
    container 'quay.io/biocontainers/protgraph:0.3.12--pyhdfd78af_0'

    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path(rust),
        path(cpp_fasta),
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path(rust),
        path("dedup_${cpp_fasta}"),
        val(max_vars), 
        val(prefix)
    )

    script:
    """
    protgraph_compact_fasta \\
        ${cpp_fasta} \\
        -o dedup_${cpp_fasta}
    """

}



process DIFF {

    publishDir("results_dedub/run_${prefix}/")
    
    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path(rust),
        path(cpp),
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path("sorted_${rust}"),
        path("sorted_${cpp}"),
        path("${prefix}_diff.txt"),
        val(max_vars), 
        val(prefix)
    )

    script:
    """
    sort -t '>' -k2 > sorted_${rust}
    sort -t '>' -k2 ${cpp} > sorted_${cpp}
    diff sorted_${rust} sorted_${cpp} > ${prefix}_diff.txt || true
    """
}

process DIFFDEDUB {
    publishDir("results/run_${prefix}/")
    
    input:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path(rust),
        path(cpp),
        val(max_vars), 
        val(prefix)
    )

    output:
    tuple(
        path(accessions_csv), 
        path(queries_csv), 
        path(rust_queries_csv),
        path(cpp_limits_csv),
        path(uniprot_txt),
        path(summary_txt),
        path(database_bpcsr),
        path("sorted_${rust}"),
        path("sorted_${cpp}"),
        path("${prefix}_diff.txt"),
        val(max_vars), 
        val(prefix)
    )

    script:
    """
    gzip -cdf ${rust} | grep -v '^>' | sort > sorted_${rust}
    grep -v '^>' ${cpp} | sort > sorted_${cpp}
    diff sorted_${rust} sorted_${cpp} > ${prefix}_diff.txt || true
    """
}

workflow {

    runs_ch = channel
        .fromPath(params.samplesheet)
        .splitCsv(header: true)
        .map { row ->
            tuple(
                file(row.input_file),
                file(row.query_file),
                row.max_vars.toInteger(),
                row.run_prefix
            )
        }

    BUILDINPUT(runs_ch)

    BUILDGRAPH(BUILDINPUT.out)
    
    READERRUST(BUILDGRAPH.out)

    READERCPP(READERRUST.out)

    DEDUPCPP(READERCPP.out)

    DIFFDEDUB(DEDUPCPP.out)
}
