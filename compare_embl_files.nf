include { BUILDINPUT } from './modules.nf'
include { BUILD_SYN_INPUT } from './modules.nf'
include { BUILDGRAPH as BUILDGRAPH }  from './modules.nf'
include { BUILDGRAPH as BUILDGRAPH_SYN } from './modules.nf'
include { READERRUST } from './modules.nf'
include { READERRUST as READERRUST_SYN } from './modules.nf'
include { DIFF } from './modules.nf'


workflow compare_embl_files {
    take:
    samplesheet

    main:

    runs_ch = channel
        .fromPath(samplesheet)
        .splitCsv(header: true)
        .map { row ->
            tuple(
                file(row.input_file),
                file(row.query_file),
                row.max_vars.toInteger(),
                row.max_cleavages.toInteger(),
                row.run_prefix
            )
        }

    BUILDINPUT(runs_ch)

    BUILD_SYN_INPUT(BUILDINPUT.out)

    // uniprot branch
    uniprot_graph = BUILDGRAPH(BUILDINPUT.out)
    READERRUST(uniprot_graph)

    // syn-embl branch
    syn_graph = BUILDGRAPH_SYN(BUILD_SYN_INPUT.out)
    READERRUST_SYN(syn_graph)

    ch_uniprot = READERRUST.out.map { _bpcsr, _queries, _limits, fasta, _max_cl, prefix -> tuple(prefix, fasta) }
    ch_syn     = READERRUST_SYN.out.map { _bpcsr, _queries, _limits, fasta, _max_cl, prefix -> tuple(prefix, fasta) }

    ch_diff = ch_uniprot.join(ch_syn)

    DIFF(ch_diff)
}
