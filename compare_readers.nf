include { BUILDINPUT } from './modules.nf'
include { BUILDGRAPH as BUILDGRAPH }  from './modules.nf'
include { READERRUST } from './modules.nf'
include { READERCPP } from './modules.nf'
include { DEDUPCPP } from './modules.nf'
include { FILTERCLEAVAGES } from './modules.nf'
include { DIFF } from './modules.nf'

workflow compare_bpcsr_readers {
    
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

    BUILDGRAPH(BUILDINPUT.out)

    READERRUST(BUILDGRAPH.out)

    READERCPP(READERRUST.out)

    DEDUPCPP(READERCPP.out)

    FILTERCLEAVAGES(DEDUPCPP.out)

    DIFF(FILTERCLEAVAGES.out)
}

