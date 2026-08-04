include { BUILDINPUT } from './modules.nf'
include { BUILD_SYN_INPUT_EBI } from './modules.nf'
include { BUILD_SYN_INPUT_UNIPROT } from './modules.nf'
include { BUILDGRAPH as BUILDGRAPH }  from './modules.nf'
include { BUILDGRAPH as BUILDGRAPH_SYN_EBI } from './modules.nf'
include { BUILDGRAPH as BUILDGRAPH_SYN_UNIPROT } from './modules.nf'
include { READERRUST } from './modules.nf'
include { READERRUST as READERRUST_SYN_EBI } from './modules.nf'
include { READERRUST as READERRUST_SYN_UNIPROT } from './modules.nf'
include { DIFF as DIFF_SYN_CANON } from './modules.nf'
include { DIFF as DIFF_SYN_SYN } from './modules.nf'


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

    BUILD_SYN_INPUT_EBI(BUILDINPUT.out)

    BUILD_SYN_INPUT_UNIPROT(BUILDINPUT.out)

    // uniprot branch
    uniprot_graph = BUILDGRAPH(BUILDINPUT.out)
    READERRUST(uniprot_graph)

    // syn-EBI branch
    syn_graph_ebi = BUILDGRAPH_SYN_EBI(BUILD_SYN_INPUT_EBI.out)
    READERRUST_SYN_EBI(syn_graph_ebi)

    // syn-UniProt branch
    syn_graph_uniprot = BUILDGRAPH_SYN_UNIPROT(BUILD_SYN_INPUT_UNIPROT.out)
    READERRUST_SYN_UNIPROT(syn_graph_uniprot)

    ch_uniprot = READERRUST.out.map { _bpcsr, _queries, _limits, fasta, _max_cl, prefix -> tuple(prefix, fasta) }
    ch_syn_uniprot     = READERRUST_SYN_UNIPROT.out.map { _bpcsr, _queries, _limits, fasta, _max_cl, prefix -> tuple(prefix, fasta) }
    ch_syn_ebi     = READERRUST_SYN_EBI.out.map { _bpcsr, _queries, _limits, fasta, _max_cl, prefix -> tuple(prefix, fasta) }

    ch_diff_syn_canon = ch_uniprot.join(ch_syn_uniprot)

    ch_diff_syn_syn = ch_syn_uniprot.join(ch_syn_ebi)

    DIFF_SYN_CANON(ch_diff_syn_canon, "SYNC-CANON")

    DIFF_SYN_SYN(ch_diff_syn_syn, "SYN_EBI-UNIPROT")
}
